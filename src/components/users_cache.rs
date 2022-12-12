use deadpool_redis::redis::{cmd, RedisResult};

use crate::utils::encrypt_string::hash_with_key;

use super::redis::RedisComponent;

const DEFAULT_EXPIRATION_TIME_SECONDS: i32 = 120;

#[async_trait::async_trait]
pub trait UsersCacheComponent {
    async fn add_user(
        &mut self,
        token: &str,
        user_id: &str,
        custom_exipry_time: Option<i32>,
    ) -> Result<(), String>;
    async fn get_user(&mut self, token: &str) -> Result<String, String>;
}

#[derive(Debug)]
pub struct UsersCache<T: RedisComponent> {
    redis_component: T,
    hashing_key: String,
}

impl<T: RedisComponent + std::fmt::Debug> UsersCache<T> {
    pub fn new(redis_component: T, hashing_key: String) -> Self {
        Self {
            redis_component,
            hashing_key,
        }
    }
}
#[async_trait::async_trait]
impl<T: RedisComponent + std::fmt::Debug + Sync + Send> UsersCacheComponent for UsersCache<T> {
    #[tracing::instrument(
        name = "Storing user in cache",
        skip(token, custom_exipry_time),
        fields(
            user_id = %user_id,
        )
    )]
    async fn add_user(
        &mut self,
        token: &str,
        user_id: &str,
        custom_exipry_time: Option<i32>,
    ) -> Result<(), String> {
        let con = self.redis_component.get_async_connection().await;

        if con.is_none() {
            let error = format!("Couldn't cache user {user_id}, redis has no connection available");
            log::error!("{}", error);
            return Err(error);
        }

        let key = hash_with_key(token, &self.hashing_key);

        let mut connection = con.unwrap();

        let set_res = cmd("SET")
            .arg(&[key.clone(), user_id.to_string()])
            .arg(&[
                "EX".to_string(),
                (custom_exipry_time.unwrap_or(DEFAULT_EXPIRATION_TIME_SECONDS)).to_string(),
            ])
            .query_async::<_, ()>(&mut connection)
            .await;

        match set_res {
            Ok(_) => Ok(()),
            Err(err) => {
                let error = format!("Couldn't cache user {err}");
                log::error!("{error}");
                Err(error)
            }
        }
    }

    async fn get_user(&mut self, token: &str) -> Result<String, String> {
        let con = self.redis_component.get_async_connection().await;

        if con.is_none() {
            log::error!("Couldn't obtain user redis has no connection available");
            return Err("Couldn't obtain user redis has no connection available".to_string());
        }

        let key = hash_with_key(token, &self.hashing_key);

        let mut connection = con.unwrap();
        let res: RedisResult<String> = cmd("GET").arg(&[key]).query_async(&mut connection).await;

        match res {
            Ok(user_id) => Ok(user_id),
            Err(err) => {
                log::debug!("User not found in cache for token {}, error {}", token, err);
                Err(err.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use deadpool_redis::{redis::RedisError, Connection};

    use mockall::mock;

    mock! {
        #[derive(Debug)]
        Redis {}

        #[async_trait]
        impl RedisComponent for Redis {
            async fn stop(&mut self) {}
            async fn run(&mut self) -> Result<(), RedisError> {}
            async fn get_async_connection(&mut self) -> Option<Connection> {
                None
            }
        }

    }

    use super::*;

    #[actix_web::test]
    async fn test_should_return_no_connection_available() -> Result<(), String> {
        let mut redis = MockRedis::new();

        let token = "my test token";
        let user_id = "joni";

        redis.expect_get_async_connection().return_once(|| None);

        let mut user_cache_component = UsersCache::new(redis, "test_key".to_string());

        let res = user_cache_component.add_user(token, user_id, None).await;

        match res {
            Ok(_) => Err("Should return the expected error".to_string()),
            Err(err) => {
                assert_eq!(
                    format!(
                        "Couldn't cache user {}, redis has no connection available",
                        user_id
                    ),
                    err
                );
                Ok(())
            }
        }
    }
}
