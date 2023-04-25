use std::future::{ready, Ready};

use actix_web::{
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;

pub struct CheckMetricsToken {
    bearer_token: String,
}

impl CheckMetricsToken {
    pub fn new(token: String) -> Self {
        CheckMetricsToken {
            bearer_token: token,
        }
    }
}

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
        ready(Ok(CheckMetricsTokenMiddleware {
            service,
            bearer_token: self.bearer_token.clone(),
        }))
    }
}
pub struct CheckMetricsTokenMiddleware<S> {
    service: S,
    bearer_token: String,
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
        if request.path() == "/metrics" {
            if self.bearer_token.is_empty() {
                log::error!("missing wkc_metrics_bearer_token in configuration component");
                let (request, _pl) = request.into_parts();

                let response = HttpResponse::InternalServerError()
                    .finish()
                    .map_into_right_body();

                return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
            }

            let token = match request
                .headers()
                .get("authorization")
                .map(|header| header.to_str())
            {
                Some(Ok(res)) => {
                    let split_header_bearer = res.split(' ').collect::<Vec<&str>>();
                    let token = split_header_bearer.get(1);
                    token.map_or("", |token| token.to_owned())
                }
                _ => "",
            };

            if token.is_empty() || token != self.bearer_token {
                let (request, _pl) = request.into_parts();

                let response = HttpResponse::Unauthorized().finish().map_into_right_body();

                return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
            }
        }

        let res = self.service.call(request);

        Box::pin(async move { res.await.map(ServiceResponse::map_into_left_body) })
    }
}
