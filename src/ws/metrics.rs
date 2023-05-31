use std::sync::Arc;

use tokio::sync::Mutex;
use warp::{http::header::HeaderValue, reject::Reject, Rejection, Reply};

use prometheus::{self, Encoder, IntCounterVec, Opts, Registry};

use super::service::mapper::error::WsServiceError;

#[derive(Clone)]
pub struct Metrics {
    pub procedure_call_collector: IntCounterVec,
    pub registry: Registry,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        let opts = Opts::new(
            "dcl_social_service_rpc_procedure_call_total",
            "Social Service RPC Websocket Procedure Calls",
        );

        let procedure_call_collector = IntCounterVec::new(opts, &["code", "procedure"])
            .expect("Metrics definition is correct, so dcl_social_service_rpc_procedure_call_total metric should be created successfully");

        let registry = Registry::new();

        Metrics {
            procedure_call_collector,
            registry,
        }
    }
}

#[derive(Debug)]
struct InvalidHeader;

impl Reject for InvalidHeader {}

pub enum Procedure {
    GetFriends,
    GetRequestEvents,
    UpdateFriendshipEvent,
    SubscribeFriendshipEventsUpdates,
}

impl Procedure {
    pub fn as_str(&self) -> &str {
        match self {
            Procedure::GetFriends => "GetFriends",
            Procedure::GetRequestEvents => "GetRequestEvents",
            Procedure::UpdateFriendshipEvent => "UpdateFriendshipEvent",
            Procedure::SubscribeFriendshipEventsUpdates => "SubscribeFriendshipEventsUpdates",
        }
    }
}

pub async fn record_procedure_call(
    metrics: Arc<Mutex<Metrics>>,
    code: Option<WsServiceError>,
    procedure: Procedure,
) {
    let code = match code {
        Some(WsServiceError::Unauthorized(_)) => "UNAUTHORIZED_ERROR",
        Some(WsServiceError::InternalServer(_)) => "INTERNAL_SERVER_ERROR",
        Some(WsServiceError::BadRequest(_)) => "BAD_REQUEST_ERROR",
        Some(WsServiceError::Forbidden(_)) => "FORBIDDEN_ERROR",
        Some(WsServiceError::TooManyRequests(_)) => "TOO_MANY_REQUESTS_ERROR",
        None => "OK",
    };

    let metrics = metrics.lock().await;

    metrics
        .procedure_call_collector
        .with_label_values(&[code, procedure.as_str()])
        .inc();
}

pub async fn register_metrics(metrics: Arc<Mutex<Metrics>>) {
    log::info!("Registering PROCEDURE_CALL_COLLECTOR");

    let metrics = metrics.lock().await;

    metrics
        .registry
        .register(Box::new(metrics.procedure_call_collector.clone()))
        .expect("Procedure Call Collector metrics should be correct, so PROCEDURE_CALL_COLLECTOR can be registered successfully");

    log::info!("[RPC] Registered PROCEDURE_CALL_COLLECTOR");
}

pub async fn metrics_handler(metrics: Arc<Mutex<Metrics>>) -> Result<impl Reply, Rejection> {
    let encoder = prometheus::TextEncoder::new();

    let metrics = metrics.lock().await;

    let mut buffer = Vec::new();
    if let Err(err) = encoder.encode(&metrics.registry.gather(), &mut buffer) {
        log::debug!(
            "[RPC] metrics_handler > Could not encode metrics for RPC WebSocket Server: {}",
            err
        );
    };

    let res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(err) => {
            log::debug!(
                "[RPC] metrics_handler > Metrics could not be from_utf8'd: {}",
                err
            );
            String::default()
        }
    };
    buffer.clear();

    Ok(res)
}

pub async fn validate_bearer_token(
    header_value: HeaderValue,
    expected_token: String,
) -> Result<(), Rejection> {
    header_value
        .to_str()
        .map_err(|_| warp::reject::custom(InvalidHeader))
        .and_then(|header_value_str| {
            let split_header_bearer = header_value_str.split(' ').collect::<Vec<&str>>();
            let token = split_header_bearer.get(1);
            let token = token.map_or("", |token| token.to_owned());

            if token == expected_token {
                Ok(())
            } else {
                Err(warp::reject::custom(InvalidHeader))
            }
        })
}
