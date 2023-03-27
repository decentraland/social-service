use dcl_rpc::transports::TransportError;

use tokio_tungstenite::{accept_async, MaybeTlsStream, WebSocketStream};

use log::{debug, error};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
    task::JoinHandle,
};

/// WebSocketServerWithAuthusing [`tokio_tungstenite`] to receive connections
///
/// With the support for Authorization
///
pub struct WebSocketServerWithAuth {
    /// Address to listen for new connection
    address: String,
    /// TPC Listener Join Handle
    tpc_listener_handle: Option<JoinHandle<()>>,
}

/// A [`WebSocketStream`] from a WebSocket connection
type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Receiver half of a channel to get notified that there is a new connection
///
/// And then attach turn the connection into a transport and attach it to the [`RpcServer`](crate::server::RpcServer)
///
type OnConnectionListener = UnboundedReceiver<Result<Socket, TransportError>>;

impl WebSocketServerWithAuth {
    /// Set the configuration and the minimum for a new WebSocket Server
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            tpc_listener_handle: None,
        }
    }

    /// Listen for new connections on the address given and do the websocket handshake in a background task
    ///
    /// Each new connection will be sent through the `OnConnectionListener`, in order to be attached to the [`RpcServer`](crate::server::RpcServer)  as a [`WebSocketTransport`]
    ///
    pub async fn listen(&mut self) -> Result<OnConnectionListener, TransportError> {
        // listen to given address
        let listener = TcpListener::bind(&self.address).await?;
        debug!("Listening on: {}", self.address);

        let (tx_on_connection_listener, rx_on_connection_listener) = unbounded_channel();

        let join_handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let peer = if let Ok(perr) = stream.peer_addr() {
                            perr
                        } else {
                            if tx_on_connection_listener
                                .send(Err(TransportError::Connection))
                                .is_err()
                            {
                                error!("WS Server: Error on sending the error to the listener")
                            }
                            continue;
                        };

                        debug!("Peer address: {}", peer);
                        let stream = MaybeTlsStream::Plain(stream);
                        if let Ok(ws) = accept_async(stream).await {
                            if tx_on_connection_listener.send(Ok(ws)).is_err() {
                                error!("WS Server: Error on sending the new ws socket to listener")
                            }
                        } else {
                            if tx_on_connection_listener
                                .send(Err(TransportError::Connection))
                                .is_err()
                            {
                                error!("WS Server: Error on sending the error to the listener")
                            }
                            continue;
                        };
                    }
                    Err(error) => {
                        if tx_on_connection_listener
                            .send(Err(TransportError::Connection))
                            .is_err()
                        {
                            error!(
                                "WS Server: Error on sending the error to the listener: {error:?}"
                            )
                        }
                    }
                }
            }
        });

        self.tpc_listener_handle = Some(join_handle);

        Ok(rx_on_connection_listener)
    }
}

impl Drop for WebSocketServerWithAuth {
    fn drop(&mut self) {
        if let Some(handle) = &self.tpc_listener_handle {
            handle.abort();
        }
    }
}
