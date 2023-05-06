use crate::{
    entities::friendship_history::FriendshipHistory,
    friendships::FriendshipServiceError,
    models::friendship_event::FriendshipEvent,
    ws::service::errors::{as_service_error, DomainErrorCode},
};

/// Validates the new event is valid and different from the last recorded.
pub fn validate_new_event(
    last_recorded_history: &Option<FriendshipHistory>,
    new_event: FriendshipEvent,
) -> Result<(), FriendshipServiceError> {
    let last_recorded_event = last_recorded_history.as_ref().map(|history| history.event);
    let is_valid = FriendshipEvent::validate_new_event_is_valid(&last_recorded_event, new_event);
    if !is_valid {
        log::error!(
            "Validate new event > Invalid friendship event: {:?}. The last recorded event is: {:?}.",
            new_event,
            last_recorded_event
        );
        return Err(as_service_error(
            DomainErrorCode::BadRequest,
            "Invalid friendship event update".to_owned(),
        ));
    };
    Ok(())
}
