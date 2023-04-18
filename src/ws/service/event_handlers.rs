use std::sync::Arc;

use crate::{
    components::database::DatabaseComponentImplementation,
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
    synapse_handlers::{
        create_or_get_synapse_room_id, store_message_in_synapse_room,
        store_room_event_in_synapse_room,
    },
    types::EventResponse,
    utils_handlers::{
        extract_update_friendship_payload, get_new_friendship_status, validate_new_event,
    },
};

pub async fn process_room_event(
    request: UpdateFriendshipPayload,
    context: Arc<SocialContext>,
    acting_user: String,
) -> Result<EventResponse, FriendshipsServiceErrorResponse> {
    let event_payload = extract_update_friendship_payload(request.clone())?;
    let new_event = event_payload.friendship_event;
    let second_user = event_payload.second_user;

    let token = request.auth_token.unwrap().synapse_token.unwrap();

    // Get the friendship info
    let db_repos = &context.db.clone().db_repos.unwrap();
    let friendships_repository = &db_repos.friendships;
    let friendship = get_friendship(friendships_repository, &acting_user, &second_user).await?;

    let synapse_room_id = create_or_get_synapse_room_id(
        friendship.as_ref(),
        &new_event,
        &acting_user,
        &second_user,
        &token,
        &context.synapse.clone(),
    )
    .await?;

    //  Get the last status from the database to later validate if the current action is valid.
    let friendship_history_repository = &db_repos.friendship_history;
    let last_recorded_history =
        get_last_history(friendship_history_repository, &friendship).await?;

    // Validate the new event is valid and different from the last recorded.
    validate_new_event(&last_recorded_history, new_event)?;

    // Get new friendship status
    let new_status = get_new_friendship_status(&acting_user, &last_recorded_history, new_event)?;

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
        room_id: synapse_room_id.as_str(),
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
    store_message_in_synapse_room(
        &token,
        synapse_room_id.as_str(),
        new_event,
        room_message_body,
        &context.synapse,
    )
    .await?;

    // Store the friendship event in the given room.
    let result = store_room_event_in_synapse_room(
        &token,
        synapse_room_id.as_str(),
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
