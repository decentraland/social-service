use async_trait::async_trait;
use deadpool_redis::{
    redis::aio::ConnectionLike,
    redis::{cmd, RedisError, RedisResult},
    Config, Pool, Runtime, Connection,
};

use super::{configuration::Redis as RedisConfig, health::Healthy};

#[derive(Clone)]
pub struct Redis {
    redis_host: String,
    pub pool: Option<Pool>,
}

#[async_trait]
pub trait RedisComponent<T: ConnectionLike> {
    async fn stop(&mut self);
    async fn run(&mut self) -> Result<(), RedisError>;
    async fn get_async_connection(&mut self) -> Option<Connection>;
}

impl Redis {
    pub fn new(config: &RedisConfig) -> Self {
        Self {
            redis_host: config.host.clone(),
            pool: None,
        }
    }
}

#[async_trait]
impl<T: ConnectionLike> RedisComponent<T> for Redis {
    async fn stop(&mut self) {
        self.pool.as_mut().unwrap().close()
    }

    async fn run(&mut self) -> Result<(), RedisError> {
        if self.pool.is_none() {
            let url = format!("redis://{}", self.redis_host);
            log::debug!("Redis URL: {}", url);

            match Config::from_url(url).create_pool(Some(Runtime::Tokio1)) {
                Ok(pool) => {
                    self.pool = Some(pool);
                }
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

    async fn get_async_connection(&mut self) -> Option<Connection> {
        match self.pool.as_mut().unwrap().get().await {
            Ok(connection) => Some(connection),
            Err(err) => {
                log::error!("Error getting connection from redis: {:?}", err);
                None
            }
        }
    }
}

#[async_trait]
impl Healthy for Redis {
    async fn is_healthy(&self) -> bool {
        match self.pool.as_ref().unwrap().get().await {
            Ok(mut con) => {
                let result: RedisResult<String> = cmd("PING").query_async(&mut con).await;
                result.is_ok()
            }
            Err(_) => false,
        }
    }
}
