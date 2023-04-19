// Responsible for managing Synapse rooms and storing events in these rooms.
use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    components::{
        synapse::{CreateRoomResponse, SynapseComponent},
        users_cache::UsersCacheComponent,
    },
    entities::friendships::Friendship,
    models::friendship_event::FriendshipEvent,
    ports::users_cache::{get_user_id_from_token, UserId},
    ws::service::utils::build_room_alias_name,
    Payload,
};

use super::errors::{FriendshipsServiceError, FriendshipsServiceErrorResponse};

/// Retrieves the User Id associated with the given Authentication Token.
///
/// If an authentication token was provided in the request, gets the
/// user id from the token and returns it as a `Result<UserId, Error>`. If no
/// authentication token was provided, returns a `Unauthorized`
/// error.
pub async fn get_user_id_from_request(
    request: &Payload,
    synapse: SynapseComponent,
    users_cache: Arc<Mutex<UsersCacheComponent>>,
) -> Result<UserId, FriendshipsServiceErrorResponse> {
    match request.synapse_token.clone() {
        // If an authentication token was provided, get the user id from the token
        Some(token) => get_user_id_from_token(synapse.clone(), users_cache.clone(), &token)
            .await
            .map_err(|_err| -> FriendshipsServiceErrorResponse {
                FriendshipsServiceError::InternalServerError.into()
            }),
        // If no authentication token was provided, return an Unauthorized error.
        None => {
            log::debug!("Get Friends > Get User ID from Token > `synapse_token` is None.");
            Err(FriendshipsServiceError::Unauthorized.into())
        }
    }
}

/// Stores a message event in a Synapse room if it's a friendship request event and the request contains a message.
pub async fn store_message_in_synapse_room<'a>(
    token: &str,
    room_id: &str,
    room_event: FriendshipEvent,
    room_message_body: Option<&str>,
    synapse: &SynapseComponent,
) -> Result<(), FriendshipsServiceErrorResponse> {
    // Check if it's a `request` event.
    if room_event != FriendshipEvent::REQUEST {
        return Ok(());
    }

    // Check if there is a message, if any, send the message event to the given room.
    if let Some(val) = room_message_body {
        // Check if the message body is not empty
        if !val.is_empty() {
            for retry_count in 0..3 {
                match synapse
                    .send_message_event_given_room(token, room_id, room_event, val)
                    .await
                {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(_err) => {
                        if retry_count == 2 {
                            return Err(FriendshipsServiceError::InternalServerError.into());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Stores a room event in a Synapse room, and it returns the `EventResponse` containing the event id if the operation was successful
pub async fn store_room_event_in_synapse_room(
    token: &str,
    room_id: &str,
    room_event: FriendshipEvent,
    room_message_body: Option<&str>,
    synapse: &SynapseComponent,
) -> Result<(), FriendshipsServiceErrorResponse> {
    let res = synapse
        .store_room_event(token, room_id, room_event, room_message_body)
        .await;

    match res {
        Ok(_) => Ok(()),
        Err(_) => Err(FriendshipsServiceError::InternalServerError.into()),
    }
}

/// Creates a new private room in Synapse and returns the `CreateRoomResponse` if successful.
/// Returns a `FriendshipsServiceErrorResponse` if there is an error communicating with Synapse.
///
/// * `token` - A `&str` representing the auth token.
/// * `user_ids` - A `Vec<&str>` containing the user ids to invite to the room. There is no need to include the current user id.
/// * `room_alias_name` -
/// * `synapse` - A reference to the `SynapseComponent` instance.
async fn create_private_room_in_synapse(
    token: &str,
    user_ids: Vec<&str>,
    room_alias_name: String,
    synapse: &SynapseComponent,
) -> Result<CreateRoomResponse, FriendshipsServiceErrorResponse> {
    let res = synapse
        .create_private_room(token, user_ids, &room_alias_name)
        .await;

    match res {
        Ok(response) => {
            let res = CreateRoomResponse {
                room_id: response.room_id,
            };
            Ok(res)
        }
        Err(_) => Err(FriendshipsServiceError::InternalServerError.into()),
    }
}

async fn get_room_id_for_alias_in_synapse(
    token: &str,
    room_alias_name: &str,
    synapse: &SynapseComponent,
) -> Result<String, FriendshipsServiceErrorResponse> {
    let res = synapse.get_room_id_for_alias(token, room_alias_name).await;

    match res {
        Ok(response) => Ok(response.room_id),
        Err(_) => Err(FriendshipsServiceError::InternalServerError.into()),
    }
}

/// Creates a new Synapse room or returns the existing room id, depending on the `Friendship` and `FriendshipEvent`.
///
/// If the `Friendship` exists, returns the `synapse_room_id` in the `Friendship` struct.
///
/// If the `Friendship` does not exist and the `FriendshipEvent` is `REQUEST`, checks if a room with the alias exists in Synapse.
/// If the room exists, returns its id.
/// If the room does not exist, a new room is created and the new room id is returned.
///
/// If the `Friendship` does not exist and the `FriendshipEvent` is not `REQUEST`, an Internal Server Error error is returned.
pub async fn get_or_create_synapse_room_id(
    friendship: Option<&Friendship>,
    new_event: &FriendshipEvent,
    acting_user: &str,
    second_user: &str,
    token: &str,
    synapse: &SynapseComponent,
) -> Result<String, FriendshipsServiceErrorResponse> {
    match friendship {
        Some(friendship) => Ok(friendship.synapse_room_id.clone()),
        None => {
            if new_event == &FriendshipEvent::REQUEST {
                let room_alias_name: String = build_room_alias_name(vec![acting_user, second_user]);

                let get_room_result =
                    get_room_id_for_alias_in_synapse(token, &room_alias_name, synapse).await;

                match get_room_result {
                    Ok(room_id) => Ok(room_id),
                    Err(_) => {
                        let create_room_result = create_private_room_in_synapse(
                            token,
                            vec![second_user],
                            room_alias_name,
                            synapse,
                        )
                        .await;

                        match create_room_result {
                            Ok(res) => Ok(res.room_id),
                            Err(_) => Err(FriendshipsServiceError::InternalServerError.into()),
                        }
                    }
                }
            } else {
                Err(FriendshipsServiceError::InternalServerError.into())
            }
        }
    }
}

/// Sets the account data event for the acting user
/// Returns `Ok(())` if the account data was successfully set, or a `FriendshipsServiceErrorResponse` if an error occurs.
pub async fn set_account_data(
    token: &str,
    acting_user: &str,
    second_user: &str,
    room_id: &str,
    synapse: &SynapseComponent,
) -> Result<(), FriendshipsServiceErrorResponse> {
    let m_direct_event = synapse.get_account_data(token, acting_user).await;

    match m_direct_event {
        Ok(m_direct_event) => {
            let mut direct_room_map = if !m_direct_event.direct.is_empty() {
                m_direct_event.direct.clone()
            } else {
                HashMap::new()
            };

            if let Some(room_ids) = direct_room_map.get_mut(second_user) {
                if room_ids.contains(&room_id.to_string()) {
                    return Ok(());
                } else {
                    direct_room_map.insert((&second_user).to_string(), vec![room_id.to_string()]);
                    synapse
                        .set_account_data(token, acting_user, direct_room_map)
                        .await
                        .map_err(|_err| FriendshipsServiceError::InternalServerError)?;
                    return Ok(());
                }
            };
            Ok(())
        }
        Err(_) => Err(FriendshipsServiceError::InternalServerError.into()),
    }
}
