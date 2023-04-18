use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    components::{
        synapse::{CreateRoomResponse, SynapseComponent},
        users_cache::UsersCacheComponent,
    },
    entities::friendships::Friendship,
    ports::{
        friendship_synapse::FriendshipEvent,
        users_cache::{get_user_id_from_token, UserId},
    },
    ws::service::utils_handlers::build_room_alias_name,
    Payload,
};

use super::{
    errors::{FriendshipsServiceError, FriendshipsServiceErrorResponse},
    types::EventResponse,
};

/// Retrieves the User Id associated with the given Authentication Token.
///
/// If an authentication token was provided in the request, this function gets the
/// user id from the token and returns it as a `Result<UserId, Error>`. If no
/// authentication token was provided, this function returns a `Unauthorized`
/// error.
///
/// * `request` -
/// * `synapse` -
/// * `users_cache` -
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

/// If it's a friendship request event and the request contains a message, we send a message event to the given room.
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

/// Stores a room event in a Synapse room, and it returns an EventResponse struct containing the event ID if the operation was successful
pub async fn store_room_event_in_synapse_room(
    token: &str,
    room_id: &str,
    room_event: FriendshipEvent,
    room_message_body: Option<&str>,
    synapse: &SynapseComponent,
) -> Result<EventResponse, FriendshipsServiceErrorResponse> {
    let res = synapse
        .store_room_event(token, room_id, room_event, room_message_body)
        .await;

    match res {
        Ok(response) => {
            let res = EventResponse {
                event_id: response.event_id,
            };
            Ok(res)
        }
        Err(_) => Err(FriendshipsServiceError::InternalServerError.into()),
    }
}

pub async fn create_private_room_in_synapse(
    token: &str,
    user_ids: Vec<&str>,
    synapse: &SynapseComponent,
) -> Result<CreateRoomResponse, FriendshipsServiceErrorResponse> {
    let room_alias_name = build_room_alias_name(user_ids.clone());

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

/// Creates or retrieves the Synapse room id.
///
/// If the Friendship does not exist and the event type is REQUEST, a new room is created
/// and the account data is set. If the Friendship does not exist and it is not a REQUEST event,
/// an Internal Server Error error is returned.
pub async fn create_or_get_synapse_room_id(
    friendship: Option<&Friendship>,
    new_event: &FriendshipEvent,
    acting_user: &str,
    second_user: &str,
    token: &str,
    synapse: &SynapseComponent,
) -> Result<String, FriendshipsServiceErrorResponse> {
    match friendship {
        Some(_friendship) => Ok("".to_string()), // TODO: friendship.room_id
        None => {
            if new_event == &FriendshipEvent::REQUEST {
                let res =
                    create_private_room_in_synapse(token, vec![acting_user, second_user], synapse)
                        .await?;

                // TODO: Set account data
                Ok(res.room_id)
            } else {
                Err(FriendshipsServiceError::InternalServerError.into())
            }
        }
    }
}
