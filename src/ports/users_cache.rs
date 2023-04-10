use std::sync::Arc;

use crate::components::{synapse::SynapseComponent, users_cache::UsersCacheComponent};
use serde::{Deserialize, Serialize};

use crate::api::routes::v1::error::CommonError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserId {
    pub social_id: String,
    pub synapse_id: String,
}

pub async fn get_user_id_from_token(
    synapse: Arc<SynapseComponent>,
    users_cache: Arc<futures_util::lock::Mutex<UsersCacheComponent>>,
    token: &String,
) -> Result<UserId, CommonError> {
    // drop mutex lock at the end of scope
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
