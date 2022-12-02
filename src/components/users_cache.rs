use deadpool_redis::redis::{cmd, RedisResult};

use super::redis::RedisComponent;

fn hash_token(token: &String) -> String {
    token.to_string()
}

struct UsersCacheComponent {
    redis_component: RedisComponent,
}

impl UsersCacheComponent {
    fn new(redis: RedisComponent) -> Self {
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
