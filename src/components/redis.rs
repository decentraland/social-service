use async_trait::async_trait;
use redis::RedisResult;

// use async_trait::async_trait;

use super::{configuration::Redis as RedisConfig, health::Healthy};
// use super::health::Healthy;

#[derive(Clone)]
pub struct RedisComponent {
    redis_host: String,
    pub client: Option<redis::Client>,
}

impl RedisComponent {
    pub fn new(config: &RedisConfig) -> Self {
        Self {
            redis_host: config.host.clone(),
            client: None,
        }
    }

    pub async fn run(&mut self) -> Result<(), redis::RedisError> {
        if self.client.is_none() {
            let url = format!("redis://{}", self.redis_host);
            log::debug!("Redis URL: {}", url);

            match redis::Client::open(url) {
                Ok(client) => self.client = Some(client),
                Err(err) => {
                    log::debug!("Error on connecting to redis: {:?}", err);
                    panic!("Unable to connect to redis {:?}", err)
                }
            };

            Ok(())
        } else {
            log::debug!("Redis Connection is already set.");
            Ok(())
        }
    }
}

#[async_trait]
impl Healthy for RedisComponent {
    async fn is_healthy(&self) -> bool {
        match self.client.as_ref().unwrap().get_async_connection().await {
            Ok(mut con) => {
                let result: RedisResult<String> = redis::cmd("PING").query_async(&mut con).await;
                result.is_ok()
            }
            Err(_) => false,
        }
    }
}
