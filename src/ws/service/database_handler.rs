// Responsible for managing friendship relationships between two users,
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    entities::{
        friendship_history::{FriendshipHistory, FriendshipHistoryRepository, FriendshipMetadata},
        friendships::{Friendship, FriendshipRepositoryImplementation, FriendshipsRepository},
    },
    models::friendship_status::FriendshipStatus,
    ws::service::{
        errors::{FriendshipsServiceError, FriendshipsServiceErrorResponse},
        types::{FriendshipPortsWs, RoomInfoWs},
    },
};

/// Retrieves a friendship relationship between two addresses
///
/// * `friendships_repository` - A reference to the `FriendshipsRepository` instance.
/// * `address_1` - The address to look for in the friendship relationship.
/// * `address_2` - The address to look for in the friendship relationship.
///
/// Returns an `Option<Friendship>` if the friendship was found, or a `FriendshipsServiceErrorResponse` if an error occurs.
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
/// Returns an `Option<FriendshipHistory>` if the last history was found, or a `FriendshipsServiceErrorResponse` if an error occurs.
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

/// Stores updates to a friendship or creates a new friendship if it does not exist.
async fn store_friendship_update(
    friendships_repository: &FriendshipsRepository,
    friendship: &Option<Friendship>,
    is_active: bool,
    address_1: &str,
    address_2: &str,
    synapse_room_id: &str,
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
                .create_new_friendships(
                    (address_1, address_2),
                    false,
                    synapse_room_id,
                    Some(transaction),
                )
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

// Updates the friendship status in the friendship table and stores an update in the friendship_history table.
pub async fn update_friendship_status<'a>(
    friendship: &'a Option<Friendship>,
    acting_user: &'a str,
    second_user: &'a str,
    new_status: FriendshipStatus,
    room_info: RoomInfoWs<'a>,
    friendship_ports: FriendshipPortsWs<'a>,
    transaction: Transaction<'static, Postgres>,
) -> Result<Transaction<'static, Postgres>, FriendshipsServiceErrorResponse> {
    // Store friendship update
    let is_active = new_status == FriendshipStatus::Friends;
    let (friendship_id_result, transaction) = store_friendship_update(
        friendship_ports.friendships_repository,
        friendship,
        is_active,
        acting_user,
        second_user,
        room_info.room_id,
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

    // Store history
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
