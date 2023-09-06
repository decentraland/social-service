use std::sync::Arc;

use crate::{
    components::database::DatabaseComponentImplementation,
    db::{
        friendships_handler::{get_friendship, get_last_history, update_friendship_status},
        types::FriendshipDbRepositories,
    },
    domain::room::RoomInfo,
    domain::{
        error::CommonError,
        event::{EventPayload, EventResponse},
        friendship_event_validator::validate_new_event,
        friendship_status_calculator::get_new_friendship_status,
    },
    synapse::synapse_handler::{
        accept_room_invitation, get_or_create_synapse_room_id, set_account_data,
        store_message_in_synapse_room, store_room_event_in_synapse_room,
    },
    ws::app::SocialContext,
};

/// Processes a friendship event update by validating it and updating the Database and Synapse.
pub async fn handle_friendship_update(
    synapse_token: String,
    event_payload: EventPayload,
    context: Arc<SocialContext>,
    acting_user: String,
) -> Result<EventResponse, CommonError> {
    let new_event = event_payload.friendship_event;
    let second_user = event_payload.second_user;

    let db_repos = context.db.clone().db_repos.ok_or_else(|| {
        log::error!("[RPC] Handle friendship update > Db repositories > `repos` is None.");
        CommonError::Unknown("".to_owned())
    })?;

    // Get the friendship info
    let friendship = get_friendship(&db_repos.friendships, &acting_user, &second_user).await?;

    let synapse_room_id = get_or_create_synapse_room_id(
        friendship.as_ref(),
        &new_event,
        &acting_user,
        &second_user,
        &synapse_token,
        &context.synapse.clone(),
    )
    .await?;

    let room_message_body = event_payload.request_event_message_body.as_deref();

    // The room may exists but maybe the current user hasn't joined it yet.
    accept_room_invitation(&synapse_token, synapse_room_id.as_str(), &context.synapse).await?;

    set_account_data(
        &synapse_token,
        &acting_user,
        &second_user,
        &synapse_room_id,
        &context.synapse,
    )
    .await?;

    //  Get the last status from the database to later validate if the current action is valid
    let last_recorded_history = get_last_history(&db_repos.friendship_history, &friendship).await?;

    // Validate the transition is valid and acting user has permission to perform it
    validate_new_event(&acting_user, &last_recorded_history, new_event)?;

    // Get new friendship status
    let new_status = get_new_friendship_status(&acting_user, new_event);

    // Start a database transaction.
    let friendship_ports = FriendshipDbRepositories {
        db: &context.db,
        friendships_repository: &db_repos.friendships,
        friendship_history_repository: &db_repos.friendship_history,
    };
    let transaction = match friendship_ports.db.start_transaction().await {
        Ok(tx) => tx,
        Err(error) => {
            log::error!("[RPC] Handle friendship update > Couldn't start transaction to store friendship update {error}");
            return Err(CommonError::Unknown("".to_owned()));
        }
    };

    // Update the friendship accordingly in the database. This means creating an entry in the friendships table or updating the is_active column.
    let room_info = RoomInfo {
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
        &synapse_token,
        synapse_room_id.as_str(),
        new_event,
        room_message_body,
        &context.synapse,
    )
    .await?;

    // Store the friendship event in the given room.
    // We'll continue storing the event in Synapse to maintain the option to rollback to Matrix without losing any friendship interaction updates
    store_room_event_in_synapse_room(
        &synapse_token,
        synapse_room_id.as_str(),
        new_event,
        room_message_body,
        &context.synapse,
    )
    .await?;

    // End transaction
    if let Err(err) = transaction.commit().await {
        log::error!(
            "[RPC] Handle friendship update > Couldn't end transaction to store friendship update {err}"
        );
        Err(CommonError::Unknown("".to_owned()))
    } else {
        Ok(EventResponse {
            user_id: second_user.to_string(),
        })
    }
}
