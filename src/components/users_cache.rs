use async_trait::async_trait;
use deadpool_redis::redis::{cmd, RedisResult};

use super::redis::RedisComponent;

fn hash_token(token: &String) -> String {
    token.to_string()
}

struct UsersCacheComponent<T: RedisComponent> {
    redis_component: T,
}

impl<T: RedisComponent> UsersCacheComponent<T> {
    fn new(redis: T) -> Self {
        Self {
            redis_component: redis,
        }
    }

    async fn add_user(&mut self, token: String, user_id: String) {
        let con = self.redis_component.get_async_connection().await;

        if con.is_none() {
            log::error!(
                "Couldn't cache user {}, redis has no connection available",
                user_id
            )
        }

        let key = hash_token(&token);

        let mut connection = con.unwrap();
        let res = cmd("SET")
            .arg(&[key, user_id])
            .query_async::<_, ()>(&mut connection)
            .await;

        match res {
            Ok(_) => {}
            Err(err) => log::error!("Couldn't cache user {}", err),
        }
    }

    async fn get_user(&mut self, token: String) -> Option<String> {
        let con = self.redis_component.get_async_connection().await;

        if con.is_none() {
            log::error!("Couldn't obtain user redis has no connection available");
            return None;
        }

        let key = hash_token(&token);

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
use super::configuration::Redis as RedisConfig;
#[cfg(test)]
use deadpool_redis::{redis::RedisError, Connection};
#[cfg(test)]
use mockall::mock;

#[cfg(test)]
mock! {
    Redis {    }

    #[async_trait]
    impl RedisComponent for Redis {
        fn new(config: &RedisConfig) -> Self{
            Self{}
        }

    async fn stop(&mut self){}
    async fn run(&mut self) -> Result<(), RedisError>{

    }
    async fn get_async_connection(&mut self) -> Option<Connection>{}
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_store_user() {
        let redis = MockRedis::new(&RedisConfig {
            host: "mock_host".to_string(),
        });

        let user_cache_component = UsersCacheComponent::new(redis);
        // assert_eq!(divide_non_zero_result(10, 2), 5);
    }

    // #[test]
    // #[should_panic]
    // fn test_any_panic() {
    //     divide_non_zero_result(1, 0);
    // }

    // #[test]
    // #[should_panic(expected = "Divide result is zero")]
    // fn test_specific_panic() {
    //     divide_non_zero_result(1, 10);
    // }
}
