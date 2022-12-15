use async_trait::async_trait;
use deadpool_redis::{
    redis::{cmd, RedisResult},
    Config, Connection, Pool, Runtime,
};

use super::{configuration::Redis as RedisConfig, health::Healthy};

#[cfg_attr(test, faux::create)]
#[derive(Clone)]
pub struct Redis {
    redis_host: String,
    pub pool: Option<Pool>,
}

#[cfg_attr(test, faux::methods)]
impl std::fmt::Debug for Redis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Redis")
            .field("redis_host", &self.redis_host)
            .field("pool", &self.pool.is_some())
            .finish()
    }
}

#[cfg_attr(test, faux::methods)]
impl Redis {
    pub fn new_and_run(config: &RedisConfig) -> Self {
        let mut redis = Self {
            redis_host: config.host.clone(),
            pool: None,
        };

        let url = format!("redis://{}", redis.redis_host);
        log::debug!("Redis URL: {}", url);

        match Config::from_url(url).create_pool(Some(Runtime::Tokio1)) {
            Ok(pool) => {
                redis.pool = Some(pool);
            }
            Err(err) => {
                log::error!("Error on connecting to redis: {:?}", err);
                panic!("Unable to connect to redis {:?}", err)
            }
        };

        redis
    }
    pub fn stop(&mut self) {
        self.pool.as_mut().unwrap().close()
    }

    pub async fn get_async_connection(&mut self) -> Option<Connection> {
        match self.pool.as_mut().unwrap().get().await {
            Ok(connection) => Some(connection),
            Err(err) => {
                log::error!("Error getting connection from redis: {:?}", err);
                None
            }
        }
    }
}

#[cfg_attr(test, faux::methods)]
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
