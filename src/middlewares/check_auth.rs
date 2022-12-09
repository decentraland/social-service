use std::{
    future::{ready, Ready},
    rc::Rc,
};

use actix_web::{
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    web::Data,
    Error, HttpMessage, HttpResponse,
};
use futures_util::future::LocalBoxFuture;

use crate::components::{app::AppComponents, synapse::SynapseComponent};

pub struct CheckAuthToken<'a> {
    auth_routes: &'a [&'a str],
}

impl<'a> CheckAuthToken<'a> {
    pub fn new(auth_routes: &'a [&str]) -> Self {
        CheckAuthToken { auth_routes }
    }
}

impl<'a, S: 'static, B> Transform<S, ServiceRequest> for CheckAuthToken<'a>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = CheckAuthTokenMiddleware<'a, S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CheckAuthTokenMiddleware {
            service: Rc::new(service),
            auth_routes: &self.auth_routes,
        }))
    }
}
pub struct CheckAuthTokenMiddleware<'a, S> {
    service: Rc<S>,
    auth_routes: &'a [&'a str],
}

const AUTH_TOKEN_HEADER: &str = "authorization";

fn is_auth_route(routes: &[&str], path: &str) -> bool {
    routes.iter().any(|x| x.to_owned() == path)
}

#[derive(Debug)]
pub struct UserId(String);

impl<'a, S: 'static, B> Service<ServiceRequest> for CheckAuthTokenMiddleware<'a, S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        if !is_auth_route(&self.auth_routes, request.path()) {
            let res = self.service.call(request);
            return Box::pin(async { res.await.map(ServiceResponse::map_into_left_body) });
        }

        let token = if let Some(header) = request.headers().get(AUTH_TOKEN_HEADER) {
            match header.to_str() {
                Ok(header) => {
                    let split_header_bearer = header.split(' ').collect::<Vec<&str>>();
                    let token = split_header_bearer.get(1);
                    if let Some(token) = token {
                        token.to_owned()
                    } else {
                        ""
                    }
                }
                Err(_) => "",
            }
        } else {
            ""
        };

        if token.is_empty() {
            let (request, _pl) = request.into_parts();

            let response = HttpResponse::BadRequest().finish().map_into_right_body();

            return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
        }

        let token = token.to_string();

        let components = request.app_data::<Data<AppComponents>>().unwrap().clone();
        let svc = self.service.clone();

        Box::pin(async move {
            let mut user_cache = components.user_cache.lock().unwrap();
            let user_id = match user_cache.get_user(&token).await {
                Ok(user_id) => user_id,
                Err(_) => match components.synapse.who_am_i(&token).await {
                    Ok(response) => {
                        if let Err(err) = user_cache.add_user(&token, &response.user_id, None).await
                        {
                            log::error!(
                                "check_auth.rs > Error on storing token into Redis: {:?}",
                                err
                            )
                        }
                        response.user_id
                    }
                    Err(_) => "".to_string(),
                },
            };

            if user_id.is_empty() {
                let (request, _pl) = request.into_parts();
                let response = HttpResponse::InternalServerError()
                    .finish()
                    .map_into_right_body();
                Ok(ServiceResponse::new(request, response))
            } else {
                let mut extensions = request.extensions_mut();
                extensions.insert(UserId(user_id));
                drop(extensions);
                let res = svc.call(request);
                res.await.map(ServiceResponse::map_into_left_body)
            }
        })
    }
}
