use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use dcl_rpc::{
    server::RpcServer,
    transports::{Transport, TransportError, TransportEvent},
};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};

use tokio::sync::Mutex;
use warp::{
    ws::{Message as WarpWSMessage, WebSocket},
    Filter,
};

use crate::{
    components::{
        configuration::Server, database::DatabaseComponent, synapse::SynapseComponent,
        users_cache::UsersCacheComponent,
    },
    ws::service::friendships_service,
    FriendshipsServiceRegistration,
};

pub struct ConfigRpcServer {
    rpc_server: Server,
}

pub struct SocialContext {
    pub synapse: Arc<SynapseComponent>,
    pub db: Arc<DatabaseComponent>,
    pub users_cache: Arc<futures_util::lock::Mutex<UsersCacheComponent>>,
    pub config: ConfigRpcServer,
}

pub async fn run_ws_transport(
    ctx: SocialContext,
) -> (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>) {
    if env_logger::try_init().is_err() {
        log::debug!("Logger already init")
    }

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

    let addr = match ctx.config.rpc_server.host.parse::<Ipv4Addr>() {
        Ok(v) => SocketAddr::new(IpAddr::V4(v), ctx.config.rpc_server.port),
        Err(err) => {
            log::debug!("Running websocket server with default values as an error was found with the configuration: {:?}", err);
            ([0, 0, 0, 0], 8085).into()
        }
    };
    let http_server_handle = tokio::spawn(async move {
        warp::serve(routes).run(addr).await;
    });

    (rpc_server_handle, http_server_handle)
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
