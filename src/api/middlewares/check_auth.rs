use std::{
    future::{ready, Ready},
    rc::Rc,
    sync::Arc,
};

use actix_web::{
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    web::Data,
    Error, HttpMessage, HttpResponse,
};
use futures_util::future::LocalBoxFuture;

use crate::{
    components::{app::AppComponents, users_cache::get_user_id_from_token},
    domain::error::CommonError,
};

pub struct CheckAuthToken {
    auth_routes: Vec<String>,
}

impl CheckAuthToken {
    pub fn new(auth_routes: Vec<String>) -> Self {
        CheckAuthToken { auth_routes }
    }
}

impl<S: 'static, B> Transform<S, ServiceRequest> for CheckAuthToken
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = CheckAuthTokenMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CheckAuthTokenMiddleware {
            service: Rc::new(service),
            auth_routes: self.auth_routes.clone(),
        }))
    }
}
pub struct CheckAuthTokenMiddleware<S> {
    service: Rc<S>,
    auth_routes: Vec<String>,
}

const AUTH_TOKEN_HEADER: &str = "authorization";

fn is_auth_route(routes: &[String], path: &str) -> bool {
    routes.iter().any(|x| *x == path)
}

#[derive(Debug)]
pub struct Token(pub String);

impl<S: 'static, B> Service<ServiceRequest> for CheckAuthTokenMiddleware<S>
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
        let matched_route = request.match_pattern();
        let is_metrics_call = request.path().eq_ignore_ascii_case("/metrics");
        if matched_route.is_none() && !is_metrics_call {
            let (request, _pl) = request.into_parts();
            let response = HttpResponse::from_error(CommonError::NotFound("".to_owned()))
                .map_into_right_body();
            return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
        }

        if is_metrics_call
            || !is_auth_route(&self.auth_routes, request.match_pattern().unwrap().as_str())
        {
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

            let response = HttpResponse::from_error(CommonError::BadRequest(
                "Missing authorization token".to_string(),
            ))
            .map_into_right_body();

            return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
        }

        let token = token.to_string();

        let svc = self.service.clone();
        Box::pin(async move {
            match request.app_data::<Data<AppComponents>>() {
                Some(components) => {
                    let user_id = get_user_id_from_token(
                        components.synapse.clone(),
                        Arc::clone(&components.users_cache),
                        &token,
                    )
                    .await;

                    if let Ok(user_id) = user_id {
                        {
                            let mut extensions = request.extensions_mut();
                            extensions.insert(user_id);
                            extensions.insert(Token(token));
                        } // drop extension

                        let res = svc.call(request);
                        res.await.map(ServiceResponse::map_into_left_body)
                    } else {
                        let (request, _pl) = request.into_parts();
                        let response =
                            HttpResponse::from_error(user_id.err().unwrap()).map_into_right_body();
                        Ok(ServiceResponse::new(request, response))
                    }
                }
                None => {
                    let (request, _pl) = request.into_parts();
                    let response = HttpResponse::from_error(CommonError::Unknown("".to_owned()))
                        .map_into_right_body();
                    Ok(ServiceResponse::new(request, response))
                }
            }
        })
    }
}
