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
    transports::{Transport, TransportError, TransportEvent},
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
};

use super::service::friendships_service;
use crate::friendships::FriendshipsServiceRegistration;
use crate::friendships::SubscribeFriendshipEventsUpdatesResponse;
use crate::notifications::Event;

pub struct ConfigRpcServer {
    pub rpc_server: Server,
}

pub struct SocialContext {
    pub synapse: SynapseComponent,
    pub db: DatabaseComponent,
    pub users_cache: Arc<Mutex<UsersCacheComponent>>,
    pub config: ConfigRpcServer,
    pub redis_publisher: Arc<RedisChannelPublisher>,
    pub redis_subscriber: Arc<RedisChannelSubscriber>,
    pub friendships_events_subscriptions:
        Arc<RwLock<HashMap<String, GeneratorYielder<SubscribeFriendshipEventsUpdatesResponse>>>>,
}

pub struct WsComponents {
    pub redis_publisher: Arc<RedisChannelPublisher>,
    pub redis_subscriber: Arc<RedisChannelSubscriber>,
    pub friendships_events_subscriptions:
        Arc<RwLock<HashMap<String, GeneratorYielder<SubscribeFriendshipEventsUpdatesResponse>>>>,
}

pub async fn init_ws_components(config: Config) -> WsComponents {
    let redis = Redis::new_and_run(&config.redis).await;
    match redis {
        Ok(redis) => {
            let redis = Arc::new(redis);
            let redis_publisher = Arc::new(init_events_channel_publisher(redis.clone()).await);
            let redis_subscriber = Arc::new(init_events_channel_subscriber(redis));
            let friendships_events_subscriptions = Arc::new(RwLock::new(HashMap::new()));
            WsComponents {
                redis_publisher,
                redis_subscriber,
                friendships_events_subscriptions,
            }
        }
        Err(err) => {
            log::error!("There was an error initializing Redis: {}", err);
            panic!("There was an error initializing Redis");
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
    let generators = ctx.friendships_events_subscriptions.clone();

    tokio::spawn(async move {
        subscribe_to_event_updates(subs, generators);
    });

    let mut rpc_server: RpcServer<SocialContext, WarpWebSocketTransport> =
        dcl_rpc::server::RpcServer::create(ctx);
    rpc_server.set_handler(|port| {
        FriendshipsServiceRegistration::register_service(
            port,
            friendships_service::MyFriendshipsService {},
        )
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

fn subscribe_to_event_updates(
    event_subscriptions: Arc<RedisChannelSubscriber>,
    generators: Arc<
        RwLock<HashMap<String, GeneratorYielder<SubscribeFriendshipEventsUpdatesResponse>>>,
    >,
) {
    let client_subscriptions = generators;
    event_subscriptions.subscribe(EVENT_UPDATES_CHANNEL_NAME, move |event_update: Event| {
        log::info!("User Update received > event_update: {event_update:?}");
        let subscriptions = client_subscriptions.clone();
        async move {
            let subs_lock = subscriptions.read().await;
            if let Some(generator) = subs_lock.get(&event_update.to.to_lowercase()) {
                if generator.r#yield(to_response(event_update)).await.is_err() {
                    log::error!("Event Update received > Couldn't send update to subscriptors");
                }
            }
        }
    });
}

fn to_response(event_update: Event) -> SubscribeFriendshipEventsUpdatesResponse {
    match event_update.friendship_event {
        Some(update) => SubscribeFriendshipEventsUpdatesResponse {
            events: [update].to_vec(),
        },
        None => {
            log::error!("There was an error when retrieving an event: Empty event");
            panic!("There was an error when retrieving an event");
        }
    }
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
    async fn receive(&self) -> Result<TransportEvent, TransportError> {
        match self.read.lock().await.next().await {
            Some(Ok(message)) => {
                if message.is_binary() {
                    let message = self.message_to_transport_event(message.into_bytes());
                    if let TransportEvent::Connect = message {
                        self.ready.store(true, Ordering::SeqCst);
                    }
                    return Ok(message);
                } else {
                    // Ignore messages that are not binary
                    return Err(TransportError::Internal);
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
        Ok(TransportEvent::Close)
    }

    async fn send(&self, message: Vec<u8>) -> Result<(), TransportError> {
        let message = WarpWSMessage::binary(message);
        self.write
            .lock()
            .await
            .send(message)
            .await
            .map_err(|_| TransportError::Internal)?;
        Ok(())
    }

    async fn close(&self) {
        match self.write.lock().await.close().await {
            Ok(_) => {
                self.ready.store(false, Ordering::SeqCst);
            }
            _ => {
                println!("Couldn't close transport")
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }
}
