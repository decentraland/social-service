use crate::{
    entities::friendship_history::FriendshipHistory, models::friendship_event::FriendshipEvent,
    ws::service::errors::FriendshipsServiceError,
};

/// Validates the new event is valid and different from the last recorded.
pub fn validate_new_event(
    last_recorded_history: &Option<FriendshipHistory>,
    new_event: FriendshipEvent,
) -> Result<(), FriendshipsServiceError> {
    let last_recorded_event = last_recorded_history.as_ref().map(|history| history.event);
    let is_valid = FriendshipEvent::validate_new_event_is_valid(&last_recorded_event, new_event);
    if !is_valid {
        return Err(FriendshipsServiceError::InternalServerError);
    };
    Ok(())
}
