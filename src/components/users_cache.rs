use deadpool_redis::redis::{cmd, RedisResult};

use super::redis::Redis;

use hex::encode;
use hmac::{Hmac, Mac};
use sha2::Sha256;

// Create alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

use std::sync::Arc;

use crate::{components::synapse::SynapseComponent, domain::error::CommonError};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

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
            social_id = %social_id,
        )
    )]
    pub async fn add_user(
        &mut self,
        token: &str,
        social_id: &str,
        synapse_id: &str,
        custom_exipry_time: Option<i32>,
    ) -> Result<(), String> {
        let con = self.redis_component.get_async_connection().await;

        if con.is_none() {
            let error =
                format!("Couldn't cache user {social_id}, redis has no connection available",);
            log::error!("{}", error);
            return Err(error);
        }

        let key = hash_with_key(token, &self.hashing_key);

        let mut connection = con.unwrap();

        let set_res = cmd("SET")
            .arg(&[
                key.clone(),
                serde_json::to_string(&UserId {
                    social_id: social_id.to_string(),
                    synapse_id: synapse_id.to_string(),
                })
                .unwrap(),
            ])
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

    pub async fn get_user(&mut self, token: &str) -> Result<UserId, String> {
        let con = self.redis_component.get_async_connection().await;

        if con.is_none() {
            log::error!("Couldn't obtain user redis has no connection available");
            return Err("Couldn't obtain user redis has no connection available".to_string());
        }

        let key = hash_with_key(token, &self.hashing_key);

        let mut connection = con.unwrap();
        let res: RedisResult<String> = cmd("GET").arg(&[key]).query_async(&mut connection).await;

        match res {
            Ok(user_id) => match serde_json::from_str::<UserId>(&user_id) {
                Ok(user_id) => Ok(user_id),
                Err(err) => Err(err.to_string()),
            },
            Err(err) => {
                log::debug!("User not found in cache for token {}, error {}", token, err);
                Err(err.to_string())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserId {
    pub social_id: String,
    pub synapse_id: String,
}
/// Retrieve the user id associated with the given token.
///
/// It first checks the user cache for the user id associated with the token.
/// If the user id is not found in the cache, it calls `who_am_i` on the `SynapseComponent` to get the user id,
/// then adds the token and user id to the cache before returning the user id.
pub async fn get_user_id_from_token(
    synapse: SynapseComponent,
    users_cache: Arc<Mutex<UsersCacheComponent>>,
    token: &String,
) -> Result<UserId, CommonError> {
    // Drop mutex lock at the end of scope.
    {
        let mut user_cache = users_cache.lock().await;
        match user_cache.get_user(token).await {
            Ok(user_id) => Ok(user_id),
            Err(e) => {
                log::info!("trying to get user {token} but {e}");
                match synapse.who_am_i(token).await {
                    Ok(response) => {
                        let user_id = UserId {
                            social_id: response.social_user_id.unwrap(),
                            synapse_id: response.user_id,
                        };

                        if let Err(err) = user_cache
                            .add_user(token, &user_id.social_id, &user_id.synapse_id, None)
                            .await
                        {
                            log::error!(
                                "Get user id from token > check_auth.rs > Error on storing token into Redis: {:?}",
                                err
                            )
                        }

                        Ok(user_id)
                    }
                    Err(err) => Err(err),
                }
            }
        }
    }
}

pub fn hash_with_key(str: &str, key: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(str.as_bytes());

    // `result` has type `CtOutput` which is a thin wrapper around array of
    // bytes for providing constant time equality check
    let result = mac.finalize();

    // To get underlying array use `into_bytes`, but be careful, since
    // incorrect use of the code value may permit timing attacks which defeats
    // the security provided by the `CtOutput`
    let code_bytes = result.into_bytes();

    encode(code_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_with_same_string() {
        // Should be deterministic for the same string twice
        assert_eq!(
            hash_with_key("test string", "test_key"),
            hash_with_key("test string", "test_key")
        );
    }

    #[test]
    fn test_hash_with_different_string() {
        // Should be deterministic for the same string twice
        assert_ne!(
            hash_with_key("test string", "test_key"),
            hash_with_key("test2 string", "test_key")
        );
    }
}
