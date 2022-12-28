use deadpool_redis::redis::{cmd, RedisResult};

use crate::utils::encrypt_string::hash_with_key;

use super::redis::Redis;

const DEFAULT_EXPIRATION_TIME_SECONDS: i32 = 1800;

#[derive(Debug)]
pub struct UsersCacheComponent {
    redis_component: Redis,
    hashing_key: String,
}

impl UsersCacheComponent {
    pub fn new(redis: Redis, hashing_key: String) -> Self {
        Self {
            redis_component: redis,
            hashing_key,
        }
    }

    #[tracing::instrument(
        name = "Storing user in cache",
        skip(token, custom_exipry_time),
        fields(
            user_id = %user_id,
        )
    )]
    pub async fn add_user(
        &mut self,
        token: &str,
        user_id: &str,
        custom_exipry_time: Option<i32>,
    ) -> Result<(), String> {
        let con = self.redis_component.get_async_connection().await;

        if con.is_none() {
            let error =
                format!("Couldn't cache user {user_id}, redis has no connection available",);
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
                log::error!("{}", error);
                Err(error)
            }
        }
    }

    pub async fn get_user(&mut self, token: &str) -> Result<String, String> {
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
