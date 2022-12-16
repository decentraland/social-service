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

use crate::{components::app::AppComponents, routes::v1::error::CommonError};

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
pub struct UserId(pub String);

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
            let response = HttpResponse::from_error(CommonError::NotFound).map_into_right_body();
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

        let components = request.app_data::<Data<AppComponents>>().unwrap().clone();
        let svc = self.service.clone();

        Box::pin(async move {
            let user_id = {
                let mut user_cache = components.users_cache.lock().unwrap();
                match user_cache.get_user(&token).await {
                    Ok(user_id) => Ok(user_id),
                    Err(_) => match components.synapse.who_am_i(&token).await {
                        Ok(response) => {
                            if let Err(err) =
                                user_cache.add_user(&token, &response.user_id, None).await
                            {
                                log::error!(
                                    "check_auth.rs > Error on storing token into Redis: {:?}",
                                    err
                                )
                            }

                            Ok(response.user_id)
                        }
                        Err(err) => Err(err),
                    },
                }
            }; // drop mutex lock at the end of scope

            if user_id.is_err() {
                let (request, _pl) = request.into_parts();
                let response =
                    HttpResponse::from_error(user_id.err().unwrap()).map_into_right_body();
                Ok(ServiceResponse::new(request, response))
            } else {
                {
                    let mut extensions = request.extensions_mut();
                    extensions.insert(UserId(user_id.unwrap()));
                } // drop extension

                let res = svc.call(request);
                res.await.map(ServiceResponse::map_into_left_body)
            }
        })
    }
}

#[cfg(test)]
mod tests {

    use actix_web::{
        web::{self, Data},
        App, HttpMessage, HttpResponse,
    };
    use faux::when;

    use crate::components::{
        app::{AppComponents, CustomComponents},
        configuration::Config,
        database::DatabaseComponent,
        redis::Redis,
        synapse::{SynapseComponent, WhoAmIResponse},
        users_cache::UsersCacheComponent,
    };

    use super::{CheckAuthToken, UserId};

    #[actix_web::test]
    async fn should_fail_without_authorization_header() {
        let cfg = Config::new().unwrap();

        let mocked_synapse = SynapseComponent::faux();
        let mocked_db = DatabaseComponent::faux();
        let mocked_users_cache = UsersCacheComponent::faux();
        let mocked_redis = Redis::faux();

        let mocked_components = CustomComponents {
            synapse: Some(mocked_synapse),
            db: Some(mocked_db),
            users_cache: Some(mocked_users_cache),
            redis: Some(mocked_redis),
        };

        let app_data = Data::new(AppComponents::new(Some(cfg), Some(mocked_components)).await);
        let opts = vec!["/need-auth".to_string()];
        // unit app to unit test middleware
        let app = actix_web::test::init_service(
            App::new()
                .app_data(app_data)
                .wrap(CheckAuthToken::new(opts))
                .route("/need-auth", web::get().to(|| HttpResponse::Accepted())),
        )
        .await;
        let req = actix_web::test::TestRequest::get()
            .uri("/need-auth")
            .to_request();

        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400)
    }

    #[actix_web::test]
    async fn should_not_call_synapse_when_token_available_in_redis() {
        let cfg = Config::new().unwrap();

        let token = "a1b2c3d4";
        let user_id = "0xa";

        let mut mocked_synapse = SynapseComponent::faux();
        let mocked_db = DatabaseComponent::faux();
        let mut mocked_users_cache = UsersCacheComponent::faux();
        let mocked_redis = Redis::faux();

        when!(mocked_users_cache.get_user).then(|_| Ok(user_id.to_string()));
        when!(mocked_users_cache.add_user).times(0);
        when!(mocked_synapse.who_am_i).times(0);

        let mocked_components = CustomComponents {
            synapse: Some(mocked_synapse),
            db: Some(mocked_db),
            users_cache: Some(mocked_users_cache),
            redis: Some(mocked_redis),
        };

        let app_data = Data::new(AppComponents::new(Some(cfg), Some(mocked_components)).await);
        let opts = vec!["/need-auth".to_string()];
        // unit app to unit test middleware
        let app = actix_web::test::init_service(
            App::new()
                .app_data(app_data)
                .wrap(CheckAuthToken::new(opts))
                .route("/need-auth", web::get().to(|| HttpResponse::Ok())),
        )
        .await;
        let header = ("authorization", format!("Bearer {}", token));

        let req = actix_web::test::TestRequest::get()
            .uri("/need-auth")
            .insert_header(header)
            .to_request();

        let resp = actix_web::test::call_service(&app, req).await;
        let extensions = resp.request().extensions();
        let ctx_user_id = extensions.get::<UserId>();
        assert_eq!(resp.status(), 200);
        assert!(ctx_user_id.is_some());
        assert_eq!(ctx_user_id.unwrap().0, user_id)
    }

    #[actix_web::test]
    async fn should_call_synapse_when_token_not_available_in_redis_and_store_userid_into_redis() {
        let cfg = Config::new().unwrap();

        let token = "a1b2c3d4";
        let user_id = "0xa";

        let mut mocked_synapse = SynapseComponent::faux();
        let mocked_db = DatabaseComponent::faux();
        let mut mocked_users_cache = UsersCacheComponent::faux();
        let mocked_redis = Redis::faux();

        when!(mocked_users_cache.get_user).then(|_| Err("".to_string()));
        when!(mocked_users_cache.add_user).then(|_| Ok(()));
        when!(mocked_users_cache.add_user).once();
        when!(mocked_synapse.who_am_i).then(|_| {
            Ok(WhoAmIResponse {
                user_id: user_id.to_string(),
            })
        });
        when!(mocked_synapse.who_am_i).once();

        let mocked_components = CustomComponents {
            synapse: Some(mocked_synapse),
            db: Some(mocked_db),
            users_cache: Some(mocked_users_cache),
            redis: Some(mocked_redis),
        };

        let app_data = Data::new(AppComponents::new(Some(cfg), Some(mocked_components)).await);
        let opts = vec!["/need-auth".to_string()];
        // unit app to unit test middleware
        let app = actix_web::test::init_service(
            App::new()
                .app_data(app_data)
                .wrap(CheckAuthToken::new(opts))
                .route("/need-auth", web::get().to(|| HttpResponse::Ok())),
        )
        .await;
        let header = ("authorization", format!("Bearer {}", token));

        let req = actix_web::test::TestRequest::get()
            .uri("/need-auth")
            .insert_header(header)
            .to_request();

        let resp = actix_web::test::call_service(&app, req).await;
        let extensions = resp.request().extensions();
        let ctx_user_id = extensions.get::<UserId>();
        assert_eq!(resp.status(), 200);
        assert!(ctx_user_id.is_some());
        assert_eq!(ctx_user_id.unwrap().0, user_id)
    }
}
