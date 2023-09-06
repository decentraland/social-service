use crate::{domain::error::CommonError, friendships::UpdateFriendshipPayload};

pub fn get_synapse_token(request: UpdateFriendshipPayload) -> Result<String, CommonError> {
    let Some(auth_token) = request.auth_token.as_ref() else {
        log::error!("[RPC] Handle friendship update > `auth_token` is missing.");
        return Err(CommonError::Unauthorized(
            "`auth_token` is missing".to_owned(),
        ));
    };

    let Some(synapse_token) = auth_token.synapse_token.as_ref() else {
        log::error!("[RPC] Handle friendship update > `synapse_token` is missing.");
        return Err(CommonError::Unauthorized(
            "`synapse_token` is missing".to_owned(),
        ));
    };
    Ok(synapse_token.to_string())
}
