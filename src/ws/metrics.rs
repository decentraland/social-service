use warp::{http::header::HeaderValue, reject::Reject, Rejection, Reply};

use lazy_static::lazy_static;
use prometheus::{self, Encoder, IntCounterVec, Opts, Registry};

use super::service::mapper::error::WsServiceError;

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

pub fn record_procedure_call(code: Option<WsServiceError>, procedure: Procedure) {
    let label = match code {
        Some(WsServiceError::Unauthorized(_)) => "UNAUTHORIZED",
        Some(WsServiceError::InternalServer(_)) => "INTERNAL_SERVER",
        Some(WsServiceError::BadRequest(_)) => "BAD_REQUEST",
        Some(WsServiceError::Forbidden(_)) => "FORBIDDEN",
        Some(WsServiceError::TooManyRequests(_)) => "TOO_MANY_REQUESTS",
        None => "OK",
    };
    PROCEDURE_CALL_COLLECTOR
        .with_label_values(&[label, procedure.as_str()])
        .inc();
}

pub fn register_metrics() {
    log::info!("Registering PROCEDURE_CALL_COLLECTOR");

    REGISTRY
        .register(Box::new(PROCEDURE_CALL_COLLECTOR.clone()))
        .expect("PROCEDURE_CALL_COLLECTOR can be registered");

    log::info!("Registered PROCEDURE_CALL_COLLECTOR");
}

pub async fn metrics_handler() -> Result<impl Reply, Rejection> {
    let encoder = prometheus::TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(err) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        log::debug!(
            "metrics_handler > Could not encode metrics for RPC WebSocket Server: {}",
            err
        );
    };

    let res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(err) => {
            log::debug!(
                "metrics_handler > Metrics could not be from_utf8'd: {}",
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

lazy_static! {
    pub static ref PROCEDURE_CALL_COLLECTOR: IntCounterVec = {
        let opts = Opts::new(
            "dcl_social_service_rpc_procedure_call",
            "Social Service RPC Websocket Procedure Calls",
        );

        IntCounterVec::new(opts, &["code", "procedure"])
            .expect("dcl_social_service_rpc_procedure_call metric can be created")
    };
    pub static ref REGISTRY: Registry = Registry::new();
}
