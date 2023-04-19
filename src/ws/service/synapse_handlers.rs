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

/// Creates a new Synapse room or returns the existing room id, depending on the `Friendship` and `FriendshipEvent`.
///
/// If the `Friendship` exists, this function returns the `room_id` in the `Friendship` struct.
///
/// If the `Friendship` does not exist and the `FriendshipEvent` is `REQUEST`, a new room is created
/// and the account data is set. The new room id is returned.
///
/// If the `Friendship` does not exist and the `FriendshipEvent` is not `REQUEST`, an Internal Server Error error is returned.
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
                let room_alias_name: String = build_room_alias_name(vec![acting_user, second_user]);
                let res = create_private_room_in_synapse(
                    token,
                    vec![second_user],
                    room_alias_name,
                    synapse,
                )
                .await;

                match res {
                    Ok(res) => {
                        synapse
                            .set_account_data(token, second_user, &res.room_id)
                            .await
                            .map_err(|_err| FriendshipsServiceError::InternalServerError)?;
                        Ok(res.room_id)
                    }
                    Err(_) => Err(FriendshipsServiceError::InternalServerError.into()),
                }
            } else {
                Err(FriendshipsServiceError::InternalServerError.into())
            }
        }
    }
}
