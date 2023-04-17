use std::sync::Arc;

use sqlx::{Postgres, Transaction};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    api::routes::synapse::room_events::FriendshipEvent,
    components::{synapse::SynapseComponent, users_cache::UsersCacheComponent},
    entities::{
        friendship_history::{
            FriendshipHistory, FriendshipHistoryRepository, FriendshipMetadata,
            FriendshipRequestEvent,
        },
        friendships::{Friendship, FriendshipRepositoryImplementation, FriendshipsRepository},
    },
    ports::users_cache::{get_user_id_from_token, UserId},
    ws::service::error::FriendshipsServiceError,
    ws::service::error::FriendshipsServiceErrorResponse,
    Payload, RequestEvents, RequestResponse, Requests, User,
};

use super::friendship_ws_types::{
    EventResponse, FriendshipPortsWs, FriendshipStatusWs, RoomInfoWs,
};

/// Retrieve the User Id associated with the given Authentication Token.
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

/// Maps a list of `FriendshipRequestEvents` to a `RequestEvents` struct.
///
/// * `requests` - A vector of `FriendshipRequestEvents` to map to `RequestResponse` struct.
/// * `user_id` - The id of the auth user.
pub fn map_request_events(requests: Vec<FriendshipRequestEvent>, user_id: String) -> RequestEvents {
    let mut outgoing_requests: Vec<RequestResponse> = Vec::new();
    let mut incoming_requests: Vec<RequestResponse> = Vec::new();

    // Iterate through each friendship request event
    for request in requests {
        // Get the user id of the acting user for the request
        let acting_user_id = request.acting_user.clone();

        // Determine the address of the other user involved in the request event
        let address = if request.address_1.eq_ignore_ascii_case(&user_id) {
            request.address_2.clone()
        } else {
            request.address_1.clone()
        };

        // Get the message (if any) associated with the request
        let message = request
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.message.clone());

        let request_response = RequestResponse {
            user: Some(User { address }),
            created_at: request.timestamp.timestamp(),
            message,
        };

        if acting_user_id.eq_ignore_ascii_case(&user_id) {
            // If the acting user is the same as the user ID, then the request is outgoing
            outgoing_requests.push(request_response);
        } else {
            // Otherwise, the request is incoming
            incoming_requests.push(request_response);
        }
    }

    // Return a RequestEvents struct containing the incoming and outgoing request lists
    RequestEvents {
        outgoing: Some(Requests {
            total: outgoing_requests.len() as i64,
            items: outgoing_requests,
        }),
        incoming: Some(Requests {
            total: incoming_requests.len() as i64,
            items: incoming_requests,
        }),
    }
}

/// Retrieves a friendship relationship between two addresses
///
/// * `friendships_repository` - A reference to the `FriendshipsRepository` instance.
/// * `address_1` - The address to look for in the friendship relationship.
/// * `address_2` - The address to look for in the friendship relationship.
///
/// Returns a `Result` with an `Option` containing the `Friendship` relationship, or a `FriendshipsServiceErrorResponse` if an error occurs.
pub async fn get_friendship(
    friendships_repository: &FriendshipsRepository,
    address_1: &str,
    address_2: &str,
) -> Result<Option<Friendship>, FriendshipsServiceErrorResponse> {
    let (friendship_result, _) = friendships_repository
        .get_friendship((address_1, address_2), None)
        .await;

    friendship_result.map_err(|_err| FriendshipsServiceError::InternalServerError.into())
}

/// Fetches the last friendship history for a given friendship
///
/// * `friendship_history_repository` - A reference to the `FriendshipHistoryRepository` instance.
/// * `friendship` - An `Option<Friendship>` to fetch the last history for.
///
/// Returns an `Option<FriendshipHistory>` if the last history was found, otherwise `Ok(None)`.
///
/// Returns a `FriendshipsServiceErrorResponse` if there was an error fetching the last history from the repository.
pub async fn get_last_history(
    friendship_history_repository: &FriendshipHistoryRepository,
    friendship: &Option<Friendship>,
) -> Result<Option<FriendshipHistory>, FriendshipsServiceErrorResponse> {
    let friendship = {
        match friendship {
            Some(friendship) => friendship,
            None => return Ok(None),
        }
    };

    let (friendship_history_result, _) = friendship_history_repository
        .get_last_history_for_friendship(friendship.id, None)
        .await;

    friendship_history_result.map_err(|_err| FriendshipsServiceError::InternalServerError.into())
}

/// Stores updates to a friendship or creates a new friendship if one does not exist.
async fn store_friendship_update(
    friendships_repository: &FriendshipsRepository,
    friendship: &Option<Friendship>,
    is_active: bool,
    address_1: &str,
    address_2: &str,
    transaction: Transaction<'static, Postgres>,
) -> (
    Result<Uuid, FriendshipsServiceErrorResponse>,
    Transaction<'static, Postgres>,
) {
    match friendship {
        Some(friendship) => {
            let (res, transaction) = friendships_repository
                .update_friendship_status(&friendship.id, is_active, Some(transaction))
                .await;

            let res = match res {
                Ok(_) => Ok(friendship.id),
                Err(err) => {
                    log::warn!("Couldn't update friendship {err}");
                    Err(FriendshipsServiceError::InternalServerError.into())
                }
            };

            (res, transaction.unwrap())
        }
        None => {
            let (friendship_id, transaction) = friendships_repository
                .create_new_friendships((address_1, address_2), false, Some(transaction))
                .await;
            (
                friendship_id.map_err(|err| {
                    log::warn!("Couldn't crate new friendship {err}");
                    FriendshipsServiceError::InternalServerError.into()
                }),
                transaction.unwrap(),
            )
        }
    }
}

pub async fn update_friendship_status<'a>(
    friendship: &'a Option<Friendship>,
    acting_user: &'a str,
    second_user: &'a str,
    new_status: FriendshipStatusWs,
    room_info: RoomInfoWs<'a>,
    friendship_ports: FriendshipPortsWs<'a>,
    transaction: Transaction<'static, Postgres>,
) -> Result<Transaction<'static, Postgres>, FriendshipsServiceErrorResponse> {
    // store friendship update
    let is_active = new_status == FriendshipStatusWs::Friends;
    let (friendship_id_result, transaction) = store_friendship_update(
        friendship_ports.friendships_repository,
        friendship,
        is_active,
        acting_user,
        second_user,
        transaction,
    )
    .await;

    let friendship_id = match friendship_id_result {
        Ok(friendship_id) => friendship_id,
        Err(err) => {
            log::error!("Couldn't store friendship update");
            let _ = transaction.rollback().await;
            return Err(err);
        }
    };

    let room_event = match serde_json::to_string(&room_info.room_event) {
        Ok(room_event_string) => room_event_string,
        Err(err) => {
            log::error!("Error serializing room event: {:?}", err);
            let _ = transaction.rollback().await;
            return Err(FriendshipsServiceError::InternalServerError.into());
        }
    };

    let metadata = room_info.room_message_body.map(|message| {
        sqlx::types::Json(FriendshipMetadata {
            message: Some(message.to_string()),
            synapse_room_id: Some(room_info.room_id.to_string()),
            migrated_from_synapse: None,
        })
    });

    // store history
    let (friendship_history_result, transaction) = friendship_ports
        .friendship_history_repository
        .create(
            friendship_id,
            &room_event,
            acting_user,
            metadata,
            Some(transaction),
        )
        .await;

    let transaction = transaction.unwrap();

    match friendship_history_result {
        Ok(_) => Ok(transaction),
        Err(err) => {
            log::error!("Couldn't store friendship history update: {:?}", err);
            let _ = transaction.rollback().await;
            Err(FriendshipsServiceError::InternalServerError.into())
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

pub async fn store_room_event_in_synapse_room(
    token: &str,
    room_id: &str,
    room_event: FriendshipEvent,
    room_message_body: Option<&str>,
    synapse: &SynapseComponent,
) -> Result<EventResponse, FriendshipsServiceErrorResponse> {
    let res = synapse
        .store_room_event(&token, room_id, room_event, room_message_body)
        .await;

    match res {
        Ok(response) => {
            let res = EventResponse {
                event_id: response.event_id,
            };
            Ok(res)
        }
        Err(_) => return Err(FriendshipsServiceError::InternalServerError.into()),
    }
}
