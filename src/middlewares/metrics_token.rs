use std::future::{ready, Ready};

use actix_web::{
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    web::Data,
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;

use crate::components::app::AppComponents;

pub struct CheckMetricsToken;
const BEARER_TOKEN_PARAM: &str = "bearer_token";

impl<S, B> Transform<S, ServiceRequest> for CheckMetricsToken
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = CheckMetricsTokenMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CheckMetricsTokenMiddleware { service }))
    }
}
pub struct CheckMetricsTokenMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for CheckMetricsTokenMiddleware<S>
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
        let query_params = qstring::QString::from(request.query_string());
        if request.path() == "/metrics" {
            let token = query_params.get(BEARER_TOKEN_PARAM).unwrap_or("");
            let app_config = request.app_data::<Data<AppComponents>>().unwrap();

            if app_config.config.wkc_metrics_bearer_token.is_empty() {
                log::error!("missing wkc_metrics_bearer_token in configuration component");
                let (request, _pl) = request.into_parts();

                let response = HttpResponse::InternalServerError()
                    .finish()
                    .map_into_right_body();

                return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
            }

            if token.is_empty() || token != app_config.config.wkc_metrics_bearer_token {
                let (request, _pl) = request.into_parts();

                let response = HttpResponse::BadRequest().finish().map_into_right_body();

                return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
            }
        }

        let res = self.service.call(request);

        Box::pin(async move { res.await.map(ServiceResponse::map_into_left_body) })
    }
}
