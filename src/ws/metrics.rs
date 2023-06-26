use std::sync::Arc;

use tokio::sync::Mutex;
use warp::{http::header::HeaderValue, reject::Reject, Rejection, Reply};

use prometheus::{
    self, Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry,
};

use crate::domain::friendship_event::FriendshipEvent;

use super::service::mapper::error::WsServiceError;

#[derive(Clone)]
pub struct Metrics {
    pub procedure_call_total_collector: IntCounterVec,
    pub connected_clients_total_collector: IntGauge,
    pub updates_sent_on_subscription_total_collector: IntCounterVec,
    pub procedure_call_size_bytes_histogram_collector: HistogramVec,
    pub registry: Registry,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

const PROCEDURE_CALL_METRIC: (&str, &str) = (
    "dcl_social_service_rpc_procedure_call_total",
    "Social Service RPC Websocket Procedure Calls",
);
const CONNECTED_CLIENTS_METRIC: (&str, &str) = (
    "dcl_social_service_rpc_connected_clients_total",
    "Social Service RPC Websocket Connected Clients",
);
const UPDATES_SENT_METRIC: (&str, &str) = (
    "dcl_social_service_rpc_updates_sent_on_subscription_total",
    "Social Service RPC Websocket Updates Sent On Subscription",
);

const PROCEDURE_CALL_SIZE: (&str, &str) = (
    "dcl_social_service_rpc_procedure_call_size_bytes_histogram",
    "Social Service RPC Websocket Procedure Call Size",
);

impl Metrics {
    pub fn new() -> Self {
        let procedure_call_total_collector =
          Self::create_int_counter_vec(PROCEDURE_CALL_METRIC, &["code", "procedure"])
          .expect("Metrics definition is correct, so dcl_social_service_rpc_procedure_call_total metric should be created successfully");

        let connected_clients_total_collector =
          Self::create_int_gauge(CONNECTED_CLIENTS_METRIC)
          .expect("Metrics definition is correct, so dcl_social_service_rpc_connected_clients_total metric should be created successfully");

        let updates_sent_on_subscription_total_collector =
          Self::create_int_counter_vec(UPDATES_SENT_METRIC, &["event"])
          .expect("Metrics definition is correct, so dcl_social_service_rpc_updates_sent_on_subscription_total metric should be created successfully");

        let procedure_call_size_bytes_histogram_collector =
          Self::create_histogram_vec(PROCEDURE_CALL_SIZE, &["procedure"])
          .expect("Metrics definition is correct, so dcl_social_service_rpc_procedure_call_size_bytes_histogram metric should be created successfully");

        let registry = Registry::new();

        Metrics {
            procedure_call_total_collector,
            connected_clients_total_collector,
            updates_sent_on_subscription_total_collector,
            procedure_call_size_bytes_histogram_collector,
            registry,
        }
    }

    fn create_int_counter_vec(
        metric: (&str, &str),
        labels: &[&str],
    ) -> Result<IntCounterVec, prometheus::Error> {
        let opts = Opts::new(metric.0, metric.1);
        IntCounterVec::new(opts, labels)
    }

    fn create_int_gauge(metric: (&str, &str)) -> Result<IntGauge, prometheus::Error> {
        IntGauge::new(metric.0, metric.1)
    }

    fn create_histogram_vec(
        metric: (&str, &str),
        labels: &[&str],
    ) -> Result<HistogramVec, prometheus::Error> {
        let opts = HistogramOpts::new(metric.0, metric.1);
        HistogramVec::new(opts, labels)
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

/// Records a procedure call. This increments the counter of procedure calls
/// based on the response code and the specific procedure.
pub async fn record_procedure_call(
    metrics: Arc<Mutex<Metrics>>,
    code: Option<WsServiceError>,
    procedure: Procedure,
) {
    let code = map_error_code(code);
    let metrics = metrics.lock().await;
    metrics
        .procedure_call_total_collector
        .with_label_values(&[code, procedure.as_str()])
        .inc();
}

/// Increments the count of connected clients.
pub async fn increment_connected_clients(metrics: Arc<Mutex<Metrics>>) {
    let metrics = metrics.lock().await;
    metrics.connected_clients_total_collector.inc();
}

/// Decrements the count of connected clients.
pub async fn decrement_connected_clients(metrics: Arc<Mutex<Metrics>>) {
    let metrics = metrics.lock().await;
    metrics.connected_clients_total_collector.dec();
}

/// Records updates sent on subscription. This increments the counter of updates sent
/// on subscription based on the event type.
pub async fn record_updates_sent(metrics: Arc<Mutex<Metrics>>, event: FriendshipEvent) {
    let metrics = metrics.lock().await;
    metrics
        .updates_sent_on_subscription_total_collector
        .with_label_values(&[event.as_str()])
        .inc();
}

/// Records the size of a procedure call. This adds the size of the procedure call to the
/// histogram for the specified procedure.
pub async fn record_procedure_call_size<T: prost::Message>(
    metrics: Arc<Mutex<Metrics>>,
    procedure: Procedure,
    msg: &T,
) {
    let metrics = metrics.lock().await;
    let size = calculate_message_size(msg);
    metrics
        .procedure_call_size_bytes_histogram_collector
        .with_label_values(&[procedure.as_str()])
        .observe(size as f64);
}

/// Calculates the size of the encoded message in bytes.
fn calculate_message_size<T: prost::Message>(msg: &T) -> usize {
    msg.encoded_len()
}

/// Maps a `WsServiceError` variant to a corresponding string representation.
fn map_error_code(code: Option<WsServiceError>) -> &'static str {
    match code {
        Some(WsServiceError::Unauthorized(_)) => "UNAUTHORIZED_ERROR",
        Some(WsServiceError::InternalServer(_)) => "INTERNAL_SERVER_ERROR",
        Some(WsServiceError::BadRequest(_)) => "BAD_REQUEST_ERROR",
        Some(WsServiceError::Forbidden(_)) => "FORBIDDEN_ERROR",
        Some(WsServiceError::TooManyRequests(_)) => "TOO_MANY_REQUESTS_ERROR",
        None => "OK",
    }
}

pub async fn register_metrics(metrics: Arc<Mutex<Metrics>>) {
    log::info!("[RPC] Registering Social Service RPC Websocket metrics");

    let metrics = metrics.lock().await;

    metrics
        .registry
        .register(Box::new(metrics.procedure_call_total_collector.clone()))
        .expect("Procedure Call Collector metrics should be correct, so PROCEDURE_CALL_COLLECTOR can be registered successfully");

    metrics
        .registry
        .register(Box::new(metrics.connected_clients_total_collector.clone()))
        .expect("Connection Total Collector metrics should be correct, so CONNECTED_CLIENTS_COLLECTOR can be registered successfully");

    metrics
        .registry
        .register(Box::new(metrics.updates_sent_on_subscription_total_collector.clone()))
            .expect("Updates Sent On Subscription Total Collector metrics should be correct, so UPDATES_SENT_ON_SUBSCRIPTION_TOTAL_COLLECTOR can be registered successfully");

    metrics
        .registry
        .register(Box::new(metrics.procedure_call_size_bytes_histogram_collector.clone()))
        .expect("Procedure Call Size Bytes Histogram Collector metrics should be correct, so PROCEDURE_CALL_SIZE_BYTES_HISTOGRAM_COLLECTOR can be registered successfully");

    log::info!("[RPC] Registered Social Service RPC Websocket metrics");
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
