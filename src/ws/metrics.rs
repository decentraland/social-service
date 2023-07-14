use std::{sync::Arc, time::Instant};

use warp::{http::header::HeaderValue, reject::Reject, Rejection, Reply};

use prometheus::{
    self, Encoder, Histogram, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry,
};

use crate::domain::friendship_event::FriendshipEvent;

use super::service::mapper::error::WsServiceError;

#[derive(Clone)]
pub struct Metrics {
    pub procedure_call_total_collector: IntCounterVec,
    pub connected_clients_total_collector: IntGauge,
    pub updates_sent_on_subscription_total_collector: IntCounterVec,
    pub in_procedure_call_size_bytes_histogram_collector: HistogramVec,
    pub out_procedure_call_size_bytes_histogram_collector: HistogramVec,
    pub procedure_call_duration_seconds_histogram_collector: HistogramVec,
    pub connection_duration_histogram_collector: Histogram,
    pub registry: Registry,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

const PROCEDURE_CALL: (&str, &str) = (
    "dcl_social_service_rpc_procedure_call_total",
    "Social Service RPC Websocket Procedure Calls",
);
const CONNECTED_CLIENTS: (&str, &str) = (
    "dcl_social_service_rpc_connected_clients_total",
    "Social Service RPC Websocket Connected Clients",
);
const UPDATES_SENT_ON_SUBSCRIPTION: (&str, &str) = (
    "dcl_social_service_rpc_updates_sent_on_subscription_total",
    "Social Service RPC Websocket Event Updates Sent On Subscription",
);

const IN_PROCEDURE_CALL_SIZE: (&str, &str) = (
    "dcl_social_service_rpc_in_procedure_call_size_bytes_histogram",
    "Social Service RPC Websocket Procedure Incoming Payload Call Size",
);

const OUT_PROCEDURE_CALL_SIZE: (&str, &str) = (
    "dcl_social_service_rpc_out_procedure_call_size_bytes_histogram",
    "Social Service RPC Websocket Procedure Outgoing Payload Call Size",
);

const PROCEDURE_CALL_DURATION: (&str, &str) = (
    "dcl_social_service_rpc_procedure_call_duration_seconds_histogram",
    "Social Service RPC Websocket Procedure Call Duration in Seconds",
);

const CONNECTION_DURATION_METRIC: (&str, &str) = (
    "dcl_social_service_rpc_connection_duration_seconds_histogram",
    "Social Service RPC WebSocket Connection Duration",
);

impl Metrics {
    pub fn new() -> Self {
        let procedure_call_total_collector =
          Self::create_int_counter_vec(PROCEDURE_CALL, &["code", "procedure"])
          .expect("Metrics definition is correct, so dcl_social_service_rpc_procedure_call_total metric should be created successfully");

        let connected_clients_total_collector =
          Self::create_int_gauge(CONNECTED_CLIENTS)
          .expect("Metrics definition is correct, so dcl_social_service_rpc_connected_clients_total metric should be created successfully");

        let updates_sent_on_subscription_total_collector =
          Self::create_int_counter_vec(UPDATES_SENT_ON_SUBSCRIPTION, &["event"])
          .expect("Metrics definition is correct, so dcl_social_service_rpc_updates_sent_on_subscription_total metric should be created successfully");

        let in_procedure_call_size_bytes_histogram_collector =
          Self::create_histogram_vec(IN_PROCEDURE_CALL_SIZE, &["procedure"])
          .expect("Metrics definition is correct, so dcl_social_service_rpc_in_procedure_call_size_bytes_histogram metric should be created successfully");

        let out_procedure_call_size_bytes_histogram_collector =
          Self::create_histogram_vec(OUT_PROCEDURE_CALL_SIZE, &["code","procedure"])
          .expect("Metrics definition is correct, so dcl_social_service_rpc_out_procedure_call_size_bytes_histogram metric should be created successfully");

        let procedure_call_duration_seconds_histogram_collector =
          Self::create_histogram_vec(PROCEDURE_CALL_DURATION, &["code", "procedure"])
          .expect("Metrics definition is correct, so dcl_social_service_rpc_procedure_call_duration_seconds_histogram metric should be created successfully");

        let connection_duration_histogram_collector =
          Self::create_histogram(CONNECTION_DURATION_METRIC)
          .expect("Metrics definition is correct, so dcl_social_service_rpc_connection_duration_seconds_histogram metric should be created successfully");

        let registry = Registry::new();

        registry
            .register(Box::new(procedure_call_total_collector.clone()))
            .expect("Procedure Call Collector metrics should be correct, so PROCEDURE_CALL can be registered successfully");

        registry
            .register(Box::new(connected_clients_total_collector.clone()))
            .expect("Connection Total Collector metrics should be correct, so CONNECTED_CLIENTS can be registered successfully");

        registry
            .register(Box::new(updates_sent_on_subscription_total_collector.clone()))
                .expect("Updates Sent On Subscription Total Collector metrics should be correct, so UPDATES_SENT_ON_SUBSCRIPTION can be registered successfully");

        registry
            .register(Box::new(in_procedure_call_size_bytes_histogram_collector.clone()))
            .expect("Procedure Request Payload Call Size Bytes Histogram Collector metrics should be correct, so IN_PROCEDURE_CALL_SIZE can be registered successfully");

        registry
            .register(Box::new(out_procedure_call_size_bytes_histogram_collector.clone()))
            .expect("Procedure Response Payload Call Size Bytes Histogram Collector metrics should be correct, so OUT_PROCEDURE_CALL_SIZE can be registered successfully");

        registry
            .register(Box::new(procedure_call_duration_seconds_histogram_collector.clone()))
            .expect("Procedure Call Duration Seconds Histogram Collector metrics should be correct, so PROCEDURE_CALL_DURATION can be registered successfully");

        registry
            .register(Box::new(connection_duration_histogram_collector.clone()))
            .expect("Connection Duration Histogram Collector metrics should be correct, so CONNECTION_DURATION_HISTOGRAM_COLLECTOR can be registered successfully");

        Metrics {
            procedure_call_total_collector,
            connected_clients_total_collector,
            updates_sent_on_subscription_total_collector,
            in_procedure_call_size_bytes_histogram_collector,
            out_procedure_call_size_bytes_histogram_collector,
            procedure_call_duration_seconds_histogram_collector,
            connection_duration_histogram_collector,
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

    fn create_histogram(metric: (&str, &str)) -> Result<Histogram, prometheus::Error> {
        let opts = HistogramOpts::new(metric.0, metric.1);
        Histogram::with_opts(opts)
    }

    /// Records a procedure call. This increments the counter of procedure calls
    /// based on the response code and the specific procedure.
    fn record_procedure_call(&self, code: Option<WsServiceError>, procedure: Procedure) {
        let code = map_error_code(code);
        self.procedure_call_total_collector
            .with_label_values(&[code, procedure.as_str()])
            .inc();
    }

    /// Increments the count of connected clients.
    pub fn increment_connected_clients(&self) {
        self.connected_clients_total_collector.inc();
    }

    /// Decrements the count of connected clients.
    pub fn decrement_connected_clients(&self) {
        self.connected_clients_total_collector.dec();
    }

    /// Records updates sent on subscription.
    /// This increments the counter of updates sent
    /// on subscription based on the event type.
    pub fn record_friendship_event_updates_sent(&self, event: FriendshipEvent) {
        self.updates_sent_on_subscription_total_collector
            .with_label_values(&[event.as_str()])
            .inc();
    }

    /// Records the size of the incoming payload of a procedure call.
    /// This adds the size of the procedure call incoming payload to the
    /// histogram for the specified procedure.
    pub fn record_in_procedure_call_size<T: prost::Message>(&self, procedure: Procedure, msg: &T) {
        let size = calculate_message_size(msg);
        self.in_procedure_call_size_bytes_histogram_collector
            .with_label_values(&[procedure.as_str()])
            .observe(size as f64);
    }

    /// Records the size of the outgoing payload of a procedure call.
    /// This adds the size of the procedure call outgoing payload to the
    /// histogram for the specified procedure and response code.
    pub fn record_out_procedure_call_size(
        &self,
        code: Option<WsServiceError>,
        procedure: Procedure,
        size: usize,
    ) {
        let code = map_error_code(code);
        self.out_procedure_call_size_bytes_histogram_collector
            .with_label_values(&[code, procedure.as_str()])
            .observe(size as f64);
    }

    /// Records the duration of a procedure call.
    /// This adds the duration of the procedure call to the
    /// histogram for the specified procedure and response code.
    fn record_request_procedure_call_duration(
        &self,
        code: Option<WsServiceError>,
        procedure: Procedure,
        start_time: Instant,
    ) {
        let code = map_error_code(code);
        let duration = Instant::now().duration_since(start_time).as_secs_f64();
        self.procedure_call_duration_seconds_histogram_collector
            .with_label_values(&[code, procedure.as_str()])
            .observe(duration);
    }

    /// Records a procedure call, its duration but not the size, useful for stream responses
    pub fn record_procedure_call_and_duration(
        &self,
        code: Option<WsServiceError>,
        procedure: Procedure,
        start_time: Instant,
    ) {
        self.record_procedure_call(code.clone(), procedure.clone());
        self.record_request_procedure_call_duration(code, procedure, start_time);
    }

    /// Records a procedure call, its duration and its outgoing payload size.
    pub fn record_procedure_call_and_duration_and_out_size(
        &self,
        code: Option<WsServiceError>,
        procedure: Procedure,
        start_time: Instant,
        size: usize,
    ) {
        self.record_procedure_call(code.clone(), procedure.clone());
        self.record_request_procedure_call_duration(code.clone(), procedure.clone(), start_time);
        self.record_out_procedure_call_size(code, procedure, size);
    }
}

#[derive(Debug)]
struct InvalidHeader;

impl Reject for InvalidHeader {}

#[derive(Clone)]
pub enum Procedure {
    GetFriends,
    GetMutualFriends,
    GetRequestEvents,
    UpdateFriendshipEvent,
    SubscribeFriendshipEventsUpdates,
}

impl Procedure {
    pub fn as_str(&self) -> &str {
        match self {
            Procedure::GetFriends => "GetFriends",
            Procedure::GetMutualFriends => "GetMutualFriends",
            Procedure::GetRequestEvents => "GetRequestEvents",
            Procedure::UpdateFriendshipEvent => "UpdateFriendshipEvent",
            Procedure::SubscribeFriendshipEventsUpdates => "SubscribeFriendshipEventsUpdates",
        }
    }
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

pub async fn metrics_handler(metrics: Arc<Metrics>) -> Result<impl Reply, Rejection> {
    let encoder = prometheus::TextEncoder::new();

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
