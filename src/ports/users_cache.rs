use std::sync::Arc;

use crate::components::app::AppComponents;
use serde::{Deserialize, Serialize};

use crate::api::routes::v1::error::CommonError;

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
    components: Arc<AppComponents>,
    token: &String,
) -> Result<UserId, CommonError> {
    // Drop mutex lock at the end of scope.
    {
        let mut user_cache = components.users_cache.lock().await;
        match user_cache.get_user(token).await {
            Ok(user_id) => Ok(user_id),
            Err(e) => {
                log::info!("trying to get user {token} but {e}");
                match components.synapse.who_am_i(token).await {
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
                                "check_auth.rs > Error on storing token into Redis: {:?}",
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
