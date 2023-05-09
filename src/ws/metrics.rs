use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use actix_http::error;
use dcl_rpc::{
    server::RpcServer,
    stream_protocol::GeneratorYielder,
    transports::{Transport, TransportError, TransportMessage},
};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};

use tokio::sync::{Mutex, RwLock};

use warp::{
    http::header::HeaderValue,
    ws::{Message as WarpWSMessage, WebSocket},
    Filter, Rejection, Reply,
};

use crate::{
    components::notifications::{
        init_events_channel_publisher, init_events_channel_subscriber, RedisChannelPublisher,
        RedisChannelSubscriber,
    },
    components::{
        configuration::{Config, Server},
        database::DatabaseComponent,
        notifications::{ChannelSubscriber, EVENT_UPDATES_CHANNEL_NAME},
        redis::Redis,
        synapse::SynapseComponent,
        users_cache::UsersCacheComponent,
    },
    domain::address::Address,
    friendships::{
        subscribe_friendship_events_updates_response, FriendshipEventResponses,
        FriendshipsServiceRegistration, SubscribeFriendshipEventsUpdatesResponse,
    },
    notifications::Event,
};

use lazy_static::lazy_static;
use prometheus::{self, Encoder, IntCounterVec, Opts, Registry};

use super::service::friendships_service;

// TODO: Check that only valid error codes are sent to prevent a huge granularity per tag
pub fn record_error_response_code(error_code: &str) {
    ERROR_RESPONSE_CODE_COLLECTOR
        .with_label_values(&[error_code])
        .inc();
}

pub fn register_metrics() {
    log::info!("Registering ERROR_RESPONSE_CODE_COLLECTOR");
    let collector = ERROR_RESPONSE_CODE_COLLECTOR.clone();

    REGISTRY
        .register(Box::new(collector))
        .expect("Collector can be registered");

    log::info!("Registered ERROR_RESPONSE_CODE_COLLECTOR");
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

lazy_static! {
    pub static ref ERROR_RESPONSE_CODE_COLLECTOR: IntCounterVec = {
        let opts = Opts::new(
            "dcl_social_service_rpc_error_response_code",
            "Social Service RPC Websocket Error Response Codes",
        );

        IntCounterVec::new(opts, &["status_code"])
            .expect("dcl_social_service_rpc_error_response_code metric can be created")
    };
    pub static ref REGISTRY: Registry = Registry::new();
}
