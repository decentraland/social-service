use async_trait::async_trait;
use deadpool_redis::redis::AsyncCommands;
use futures_util::{Future, StreamExt as _};
use log::{debug, error};
pub use prost::Message as ProtocolMessage;
use std::sync::Arc;

use crate::components::redis::Redis;

pub trait ChannelSubscriber: Send + Sync {
    fn subscribe<NewPublishment: ProtocolMessage + Default, U: Future<Output = ()> + Send + Sync>(
        &self,
        channel_name: &str,
        on_update_fn: impl Fn(NewPublishment) -> U + Send + Sync + 'static,
    );
}

#[async_trait]
pub trait ChannelPublisher<Publishment>: Send + Sync {
    async fn publish(&self, update: Publishment);
}

pub struct RedisChannelSubscriber {
    redis: Arc<Redis>,
}

impl RedisChannelSubscriber {
    pub(crate) fn new(redis: Arc<Redis>) -> Self {
        Self { redis }
    }
}

impl ChannelSubscriber for RedisChannelSubscriber {
    /// Listens to a specific channel for new messages
    fn subscribe<
        NewPublishment: ProtocolMessage + Default,
        U: Future<Output = ()> + Send + Sync,
    >(
        &self,
        channel_name: &str,
        on_update_fn: impl Fn(NewPublishment) -> U + Send + Sync + 'static,
    ) {
        let redis = self.redis.clone(); // Should we have an Option to do an Option::take instead of clonning and leaving a useless and unused Arc instance?
        let channel_name = channel_name.to_string();
        tokio::spawn(async move {
            debug!("Subscribing to channel {channel_name}");
            let connection = redis
                .get_async_connection()
                .await
                .expect("to get a connection"); // TODO: Error handling

            let connection = deadpool_redis::Connection::take(connection);
            let mut pubsub = connection.into_pubsub();
            pubsub
                .subscribe(channel_name.clone())
                .await
                .expect("to be able to listen to this channel");

            debug!("Subscribed to channel {channel_name}!");
            let mut on_message_stream = pubsub.on_message();

            loop {
                match on_message_stream.next().await {
                    Some(message) => {
                        let payload = message.get_payload::<Vec<u8>>();
                        match payload {
                            Ok(payload) => {
                                debug!("New message received from channel");
                                let update = NewPublishment::decode(&*payload);
                                match update {
                                    Ok(update) => {
                                        debug!("New publishment parsed {update:?}");
                                        on_update_fn(update).await;
                                    }
                                    Err(_) => error!("Couldn't deserialize update"),
                                }
                            }
                            Err(_) => error!("Couldn't retrieve payload"),
                        }
                    }
                    None => debug!("Couldn't read a message from stream"),
                }
            }
        });
    }
}

pub struct RedisChannelPublisher {
    redis: Arc<Redis>,
    channel_name: String,
}

impl RedisChannelPublisher {
    pub fn new(redis: Arc<Redis>, channel_name: &str) -> Self {
        Self {
            redis,
            channel_name: channel_name.to_string(),
        }
    }
}

#[async_trait]
impl<Publishment: ProtocolMessage + 'static> ChannelPublisher<Publishment>
    for RedisChannelPublisher
{
    async fn publish(&self, publishment: Publishment) {
        debug!("Publish > Getting connection...");
        let mut publish = self
            .redis
            .get_async_connection()
            .await
            .expect("to get a connection"); // TODO: Handle error

        debug!("Publish > Encoding message...");
        let publishment_bin = publishment.encode_to_vec();

        debug!("Publish > Publishing...");
        let result: Result<usize, _> = publish.publish(&self.channel_name, publishment_bin).await;
        match result {
            Ok(result) => debug!("Publish > Done with response {result}"),
            Err(e) => error!("Couldn't publish message with error: {e:?}"),
        }
    }
}

pub const EVENT_UPDATES_CHANNEL_NAME: &str = "FRIENDSHIP_EVENTS_UPDATES";

pub fn init_events_channel_subscriber(redis: Arc<Redis>) -> RedisChannelSubscriber {
    RedisChannelSubscriber::new(redis)
}

pub async fn init_events_channel_publisher(redis: Arc<Redis>) -> RedisChannelPublisher {
    RedisChannelPublisher::new(redis, EVENT_UPDATES_CHANNEL_NAME)
}
