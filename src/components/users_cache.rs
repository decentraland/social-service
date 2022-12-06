use deadpool_redis::redis::{cmd, RedisResult};

use crate::utils::encrypt_string::hash_with_key;

use super::redis::RedisComponent;

pub struct UsersCacheComponent<T: RedisComponent> {
    redis_component: T,
    hashing_key: String,
}

impl<T: RedisComponent> UsersCacheComponent<T> {
    pub fn new(redis: T, hashing_key: String) -> Self {
        Self {
            redis_component: redis,
            hashing_key,
        }
    }

    pub async fn add_user(&mut self, token: String, user_id: String) -> Result<(), String> {
        let con = self.redis_component.get_async_connection().await;

        if con.is_none() {
            let error = format!(
                "Couldn't cache user {}, redis has no connection available",
                user_id
            );
            log::error!("{}", error);
            return Err(error);
        }

        let key = hash_with_key(token.clone(), self.hashing_key.clone());

        let mut connection = con.unwrap();
        let res = cmd("SET")
            .arg(&[key, user_id])
            .query_async::<_, ()>(&mut connection)
            .await;

        match res {
            Ok(_) => Ok(()),
            Err(err) => {
                let error = format!("Couldn't cache user {}", err);
                log::error!("{}", error);
                Err(error)
            }
        }
    }

    pub async fn get_user(&mut self, token: String) -> Option<String> {
        let con = self.redis_component.get_async_connection().await;

        if con.is_none() {
            log::error!("Couldn't obtain user redis has no connection available");
            return None;
        }

        let key = hash_with_key(token.clone(), self.hashing_key.clone());

        let mut connection = con.unwrap();
        let res: RedisResult<String> = cmd("GET").arg(&[key]).query_async(&mut connection).await;

        match res {
            Ok(user_id) => Some(user_id),
            Err(err) => {
                log::debug!("User not found in cache for token {}, error {}", token, err);
                None
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

        let mut user_cache_component = UsersCacheComponent::new(redis, "test_key".to_string());

        let res = user_cache_component
            .add_user(token.to_string(), user_id.to_string())
            .await;

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
