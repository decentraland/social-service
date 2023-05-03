use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

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
    ws::{Message as WarpWSMessage, WebSocket},
    Filter,
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
    models::address::Address,
};

use super::service::friendships_service;
use crate::friendships::FriendshipsServiceRegistration;
use crate::friendships::SubscribeFriendshipEventsUpdatesResponse;
use crate::notifications::Event;

pub struct ConfigRpcServer {
    pub rpc_server: Server,
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
}

pub struct WsComponents {
    pub redis_publisher: Arc<RedisChannelPublisher>,
    pub redis_subscriber: Arc<RedisChannelSubscriber>,
    pub friendships_events_generators:
        Arc<RwLock<HashMap<Address, GeneratorYielder<SubscribeFriendshipEventsUpdatesResponse>>>>,
    pub transport_context: Arc<RwLock<HashMap<TransportId, SocialTransportContext>>>,
}

pub async fn init_ws_components(config: Config) -> WsComponents {
    let redis = Redis::new_and_run(&config.redis).await;
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
            }
        }
        Err(err) => {
            panic!("There was an error initializing Redis for Pub/Sub: {}", err);
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
    let generators_clone = ctx.friendships_events_generators.clone();
    let transport_contexts = ctx.transport_context.clone();

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
            let transport_contexts_lock = transport_contexts_clone.read().await;
            if let Some(transport_ctx) = transport_contexts_lock.get(&transport_id) {
                generators_clone
                    .write()
                    .await
                    .remove(&transport_ctx.address);
            };
            let mut transport_contexts_lock = transport_contexts_clone.write().await;
            transport_contexts_lock.remove(&transport_id);
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
    let routes = warp::get().and(rpc_route.or(rest_routes));

    let http_server_handle = tokio::spawn(async move {
        log::info!("Running RPC WebSocket Server at 0.0.0.:{}", port);
        warp::serve(routes).run(([0, 0, 0, 0], port)).await;
    });

    (rpc_server_handle, http_server_handle)
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
            events: [update].to_vec(),
        })
}

type ReadStream = SplitStream<WebSocket>;
type WriteStream = SplitSink<WebSocket, WarpWSMessage>;

pub struct WarpWebSocketTransport {
    read: Mutex<ReadStream>,
    write: Mutex<WriteStream>,
    ready: AtomicBool,
}

impl WarpWebSocketTransport {
    /// Crates a new [`WebSocketTransport`] from a Websocket connection generated by [`WebSocketServer`] or [`WebSocketClient`]
    pub fn new(ws: WebSocket) -> Self {
        let (write, read) = ws.split();
        Self {
            read: Mutex::new(read),
            write: Mutex::new(write),
            ready: AtomicBool::new(false),
        }
    }
}

#[async_trait::async_trait]
impl Transport for WarpWebSocketTransport {
    async fn receive(&self) -> Result<TransportMessage, TransportError> {
        match self.read.lock().await.next().await {
            Some(Ok(message)) => {
                if message.is_binary() {
                    let message_data = message.into_bytes();
                    return Ok(message_data);
                } else {
                    // Ignore messages that are not binary
                    log::error!("> WebSocketTransport > Received message is not binary");
                    return Err(TransportError::NotBinaryMessage);
                }
            }
            Some(Err(err)) => {
                println!("Failed to receive message {err:?}");
            }
            None => {
                println!("No message")
            }
        }
        println!("Closing transport...");
        self.close().await;
        Err(TransportError::Closed)
    }

    async fn send(&self, message: Vec<u8>) -> Result<(), TransportError> {
        let message = WarpWSMessage::binary(message);
        match self.write.lock().await.send(message).await {
            Err(err) => {
                log::error!(
                    "> WebSocketTransport > Error on sending in a ws connection {}",
                    err.to_string()
                );

                let error = TransportError::Internal(Box::new(err));

                Err(error)
            }
            Ok(_) => Ok(()),
        }
    }

    async fn close(&self) {
        match self.write.lock().await.close().await {
            Ok(_) => {
                self.ready.store(false, Ordering::SeqCst);
            }
            _ => {
                log::error!("Couldn't close transport")
            }
        }
    }
}
