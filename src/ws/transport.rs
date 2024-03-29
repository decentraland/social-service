use std::sync::atomic::{AtomicBool, Ordering};

use dcl_rpc::transports::{Transport, TransportError, TransportMessage};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};

use tokio::sync::Mutex;

use warp::ws::{Message as WarpWSMessage, WebSocket};

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
                    log::error!("[RPC] WebSocketTransport > Received message is not binary");
                    return Err(TransportError::NotBinaryMessage);
                }
            }
            Some(Err(err)) => {
                log::error!("[RPC] Failed to receive message {err:?}");
            }
            None => {
                log::error!("[RPC] No message");
            }
        }
        log::info!("[RPC] Closing transport...");
        self.close().await;
        Err(TransportError::Closed)
    }

    async fn send(&self, message: Vec<u8>) -> Result<(), TransportError> {
        let message = WarpWSMessage::binary(message);
        match self.write.lock().await.send(message).await {
            Err(err) => {
                log::error!(
                    "[RPC] WebSocketTransport > Error on sending in a ws connection {}",
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
                log::error!("[RPC] Couldn't close transport")
            }
        }
    }
}
