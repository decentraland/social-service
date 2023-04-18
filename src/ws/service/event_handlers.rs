use std::sync::Arc;

use crate::{
    components::database::DatabaseComponentImplementation,
    ports::friendship_synapse::FriendshipEvent,
    ws::{
        app::SocialContext,
        service::{
            database_handlers::{get_last_history, update_friendship_status},
            errors::FriendshipsServiceError,
            types::{FriendshipPortsWs, RoomInfoWs},
        },
    },
    UpdateFriendshipPayload,
};

use super::{
    database_handlers::get_friendship,
    errors::FriendshipsServiceErrorResponse,
    synapse_handlers::{store_message_in_synapse_room, store_room_event_in_synapse_room},
    types::EventResponse,
    utils_handlers::{extract_event_payload, get_friendship_status},
};

pub async fn process_room_event(
    request: UpdateFriendshipPayload,
    context: Arc<SocialContext>,
    user_id: String,
) -> Result<EventResponse, FriendshipsServiceErrorResponse> {
    let event_payload = extract_event_payload(request.clone())?;

    let new_event = event_payload.friendship_event;

    let acting_user = user_id;
    let second_user = event_payload.second_user;

    // Get the friendship info
    let db_repos = &context.db.clone().db_repos.unwrap();
    let friendships_repository = &db_repos.friendships;
    let friendship = get_friendship(friendships_repository, &acting_user, &second_user).await?;

    // TODO: If there is no existing Friendship and the event type is REQUEST, create a new room.
    // TODO: If there is no existing Friendship and it is not a REQUEST Event, return an Invalid Action error.
    let (friendship, synapse_room_id) = match friendship {
        Some(friendship) => (Some(friendship), ""), // TODO: friendship.room_id
        None => {
            if new_event == FriendshipEvent::REQUEST {
                // TODO: Create room
                let room_id = "";
                (None, room_id)
            } else {
                return Err(FriendshipsServiceError::InternalServerError.into());
            }
        }
    };

    //  Get the last status from the database to later validate if the current action is valid.
    let friendship_history_repository = &db_repos.friendship_history;
    let last_recorded_history =
        get_last_history(friendship_history_repository, &friendship).await?;

    // Validate if the new status that is trying to be set is valid. If it's invalid or it has not changed, return here.
    let last_recorded_event = { last_recorded_history.as_ref().map(|history| history.event) };
    let is_valid = FriendshipEvent::validate_new_event_is_valid(&last_recorded_event, new_event);
    if !is_valid {
        return Err(FriendshipsServiceError::InternalServerError.into());
    };

    // Get new friendship status
    let new_status = get_friendship_status(&acting_user, &last_recorded_history, new_event)?;

    // Start a database transaction.
    let friendship_ports = FriendshipPortsWs {
        db: &context.db,
        friendships_repository: &db_repos.friendships,
        friendship_history_repository: &db_repos.friendship_history,
    };
    let transaction = match friendship_ports.db.start_transaction().await {
        Ok(tx) => tx,
        Err(error) => {
            log::error!("Couldn't start transaction to store friendship update {error}");
            return Err(FriendshipsServiceError::InternalServerError.into());
        }
    };

    // Update the friendship accordingly in the database. This means creating an entry in the friendships table or updating the is_active column.
    let room_message_body = event_payload.request_event_message_body.as_deref();
    let room_info = RoomInfoWs {
        room_event: new_event,
        room_message_body,
        room_id: synapse_room_id,
    };
    let transaction = update_friendship_status(
        &friendship,
        &acting_user,
        &second_user,
        new_status,
        room_info,
        friendship_ports,
        transaction,
    )
    .await?;

    // If it's a friendship request event and the request contains a message, send a message event to the given room.
    let token = request.auth_token.unwrap().synapse_token.unwrap();
    store_message_in_synapse_room(
        &token,
        synapse_room_id,
        new_event,
        room_message_body,
        &context.synapse,
    )
    .await?;

    // Store the friendship event in the given room.
    let result = store_room_event_in_synapse_room(
        &token,
        synapse_room_id,
        new_event,
        room_message_body,
        &context.synapse,
    )
    .await;

    match result {
        // TODO: handle different event responses
        Ok(value) => {
            // End transaction
            let transaction_result = transaction.commit().await;

            match transaction_result {
                Ok(_) => Ok(value),
                Err(_) => Err(FriendshipsServiceError::InternalServerError.into()),
            }
        }
        Err(_err) => Err(FriendshipsServiceError::InternalServerError.into()),
    }
}
