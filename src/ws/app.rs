use std::{collections::HashMap, sync::Arc};

use dcl_rpc::{server::RpcServer, stream_protocol::GeneratorYielder};

use tokio::sync::{Mutex, RwLock};

use warp::{http::header::HeaderValue, Filter};

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

use super::{
    metrics::{metrics_handler, register_metrics, validate_bearer_token, Metrics},
    service::friendships_service,
    transport::WarpWebSocketTransport,
};

pub struct ConfigRpcServer {
    pub rpc_server: Server,
    pub wkc_metrics_bearer_token: String,
}

pub struct SocialTransportContext {
    pub address: Address,
}

type TransportId = u32;

pub struct SocialContext {
    pub synapse: SynapseComponent,
    pub db: DatabaseComponent,
    pub users_cache: Arc<Mutex<UsersCacheComponent>>,
    pub config: ConfigRpcServer,
    pub redis_publisher: Arc<RedisChannelPublisher>,
    pub redis_subscriber: Arc<RedisChannelSubscriber>,
    pub friendships_events_generators:
        Arc<RwLock<HashMap<Address, GeneratorYielder<SubscribeFriendshipEventsUpdatesResponse>>>>,
    pub transport_context: Arc<RwLock<HashMap<TransportId, SocialTransportContext>>>,
    pub friends_stream_page_size: u16,
    pub metrics: Arc<Mutex<Metrics>>,
}

pub struct WsComponents {
    pub redis_publisher: Arc<RedisChannelPublisher>,
    pub redis_subscriber: Arc<RedisChannelSubscriber>,
    pub friendships_events_generators:
        Arc<RwLock<HashMap<Address, GeneratorYielder<SubscribeFriendshipEventsUpdatesResponse>>>>,
    pub transport_context: Arc<RwLock<HashMap<TransportId, SocialTransportContext>>>,
    pub metrics: Arc<Mutex<Metrics>>,
}

pub async fn init_ws_components(config: Config) -> WsComponents {
    let redis = Redis::new_and_run(&config.redis).await;

    let metrics = Arc::new(Mutex::new(Metrics::new()));

    match redis {
        Ok(redis) => {
            let redis = Arc::new(redis);
            let redis_publisher = Arc::new(init_events_channel_publisher(redis.clone()).await);
            let redis_subscriber = Arc::new(init_events_channel_subscriber(redis));
            let friendships_events_generators = Arc::new(RwLock::new(HashMap::new()));
            let transport_context = Arc::new(RwLock::new(HashMap::new()));
            WsComponents {
                redis_publisher,
                redis_subscriber,
                friendships_events_generators,
                transport_context,
                metrics,
            }
        }
        Err(err) => {
            panic!("There was an error initializing Redis for Pub/Sub: {err}");
        }
    }
}

pub async fn run_ws_transport(
    ctx: SocialContext,
) -> (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>) {
    if env_logger::try_init().is_err() {
        log::debug!("Logger already init")
    }
    let port = ctx.config.rpc_server.port;
    let subs = ctx.redis_subscriber.clone();
    let generators = ctx.friendships_events_generators.clone();
    let wkc_metrics_bearer_token = ctx.config.wkc_metrics_bearer_token.clone();
    let generators_clone = ctx.friendships_events_generators.clone();
    let transport_contexts = ctx.transport_context.clone();
    let metrics = ctx.metrics.clone();

    tokio::spawn(async move {
        subscribe_to_event_updates(subs, generators.clone());
    });

    let mut rpc_server: RpcServer<SocialContext, WarpWebSocketTransport> =
        dcl_rpc::server::RpcServer::create(ctx);
    rpc_server.set_module_registrator_handler(|port| {
        FriendshipsServiceRegistration::register_service(
            port,
            friendships_service::MyFriendshipsService {},
        )
    });
    rpc_server.set_on_transport_closes_handler(move |_, transport_id| {
        let transport_contexts_clone = transport_contexts.clone();
        let generators_clone = generators_clone.clone();
        tokio::spawn(async move {
            remove_transport_id_from_context(
                transport_id,
                transport_contexts_clone,
                generators_clone,
            )
            .await;
        });
    });

    // Get the Server Events Sender
    let server_events_sender = rpc_server.get_server_events_sender();

    let rpc_server_handle = tokio::spawn(async move {
        rpc_server.run().await;
    });

    let rpc_route = warp::path::end()
        // Check if the connection wants to be upgraded to have a WebSocket Connection.
        .and(warp::ws())
        // Get the connection and set a callback to send the WebSocket Transport to the RpcServer once the connection is finally upgraded.
        .map(move |ws: warp::ws::Ws| {
            let server_events_sender = server_events_sender.clone();
            ws.on_upgrade(|websocket| async move {
                server_events_sender
                    .send_attach_transport(Arc::new(WarpWebSocketTransport::new(websocket)))
                    .unwrap();
            })
        });

    let rest_routes = warp::path("health")
        .and(warp::path("live"))
        .and(warp::path::end())
        .map(|| "\"alive\"".to_string());

    // Register metrics
    register_metrics(metrics.clone()).await;

    // Metrics route
    let metrics_route = warp::path!("metrics")
        .and(warp::path::end())
        .and(warp::header::value("authorization"))
        .and_then(move |header_value: HeaderValue| {
            let expected_token = wkc_metrics_bearer_token.clone();
            validate_bearer_token(header_value, expected_token)
        })
        .untuple_one()
        .and(warp::any().map(move || Arc::clone(&metrics)))
        .and_then(metrics_handler);

    let routes = warp::get().and(rpc_route.or(rest_routes).or(metrics_route));

    let http_server_handle = tokio::spawn(async move {
        log::info!("Running RPC WebSocket Server at 0.0.0.:{}", port);
        warp::serve(routes).run(([0, 0, 0, 0], port)).await;
    });

    (rpc_server_handle, http_server_handle)
}

async fn remove_transport_id_from_context(
    transport_id: TransportId,
    transport_contexts: Arc<RwLock<HashMap<TransportId, SocialTransportContext>>>,
    generators: Arc<
        RwLock<HashMap<Address, GeneratorYielder<SubscribeFriendshipEventsUpdatesResponse>>>,
    >,
) {
    let transport_contexts_read_lock = transport_contexts.read().await;
    if let Some(transport_ctx) = transport_contexts_read_lock.get(&transport_id) {
        // First remove the generators of the corresponding address
        generators.write().await.remove(&transport_ctx.address);
    };
    drop(transport_contexts_read_lock);
    let mut transport_contexts_write_lock = transport_contexts.write().await;
    transport_contexts_write_lock.remove(&transport_id);
}

// Subscribe to Redis Pub/Sub to listen on friendship events updates, so then can notify the affected users on their corresponding generators
fn subscribe_to_event_updates(
    event_subscriptions: Arc<RedisChannelSubscriber>,
    client_generators: Arc<
        RwLock<HashMap<Address, GeneratorYielder<SubscribeFriendshipEventsUpdatesResponse>>>,
    >,
) {
    event_subscriptions.subscribe(EVENT_UPDATES_CHANNEL_NAME, move |event_update: Event| {
        log::debug!("User Update received > event_update: {event_update:?}");
        let generators = client_generators.clone();
        async move {
            send_update_to_corresponding_generator(generators, event_update).await;
        }
    });
}

async fn send_update_to_corresponding_generator(
    generators: Arc<
        RwLock<HashMap<Address, GeneratorYielder<SubscribeFriendshipEventsUpdatesResponse>>>,
    >,
    event_update: Event,
) {
    if let Some(response) = event_as_friendship_update_response(event_update.clone()) {
        let corresponding_user_id = Address(event_update.to.to_lowercase());

        let generators_lock = generators.read().await;

        if let Some(generator) = generators_lock.get(&corresponding_user_id) {
            if generator.r#yield(response.clone()).await.is_err() {
                log::error!("Event Update received > Couldn't send update to subscriptors. Update: {:?}, Subscriptor: {:?}", response, &corresponding_user_id);
            }
        }
    }
}

fn event_as_friendship_update_response(
    event_update: Event,
) -> Option<SubscribeFriendshipEventsUpdatesResponse> {
    event_update
        .friendship_event
        .map(|update| SubscribeFriendshipEventsUpdatesResponse {
            response: Some(
                subscribe_friendship_events_updates_response::Response::Events(
                    FriendshipEventResponses {
                        responses: [update].to_vec(),
                    },
                ),
            ),
        })
}
