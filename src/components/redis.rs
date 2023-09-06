use async_trait::async_trait;
use deadpool_redis::{
    redis::{cmd, RedisResult},
    Connection, CreatePoolError, Pool, Runtime,
};

use super::{configuration::RedisConfig, health::Healthy};

#[derive(Clone)]
pub struct Redis {
    redis_host: String,
    pub pool: Pool,
}

impl std::fmt::Debug for Redis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Redis")
            .field("redis_host", &self.redis_host)
            .field("pool", &self.pool.status())
            .finish()
    }
}

impl Redis {
    pub async fn new_and_run(config: &RedisConfig) -> Result<Self, CreatePoolError> {
        let url = format!("redis://{}", config.host.clone());
        log::info!("Connecting to Redis at URL: {}", url);

        let pool = deadpool_redis::Config::from_url(url).create_pool(Some(Runtime::Tokio1))?;
        let conn = pool.get().await;

        if let Err(err) = conn {
            log::error!("Error on connecting to redis: {:?}", err);
            panic!("Unable to connect to redis {err:?}")
        }

        Ok(Redis {
            redis_host: config.host.clone(),
            pool,
        })
    }

    pub fn stop(&self) {
        self.pool.close()
    }

    pub async fn get_async_connection(&self) -> Option<Connection> {
        match self.pool.get().await {
            Ok(connection) => Some(connection),
            Err(err) => {
                log::error!("Error getting connection from redis: {:?}", err);
                None
            }
        }
    }

    pub async fn ping(&self) -> bool {
        match self.get_async_connection().await {
            None => false,
            Some(mut conn) => {
                let result: RedisResult<String> = cmd("PING").query_async(&mut conn).await;
                result.is_ok()
            }
        }
    }
}

#[async_trait]
impl Healthy for Redis {
    async fn is_healthy(&self) -> bool {
        self.ping().await
    }
}
