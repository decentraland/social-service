use crate::{
    entities::{
        friendship_event::FriendshipEvent, friendship_history::FriendshipHistory,
        friendship_status::FriendshipStatus,
    },
    ws::service::errors::FriendshipsServiceError,
    ws::service::errors::FriendshipsServiceErrorResponse,
};

/// Calculates the new friendship status based on the provided friendship event and the last recorded history.
pub fn get_new_friendship_status(
    acting_user: &str,
    last_recorded_history: &Option<FriendshipHistory>,
    room_event: FriendshipEvent,
) -> Result<FriendshipStatus, FriendshipsServiceErrorResponse> {
    match room_event {
        FriendshipEvent::REQUEST => {
            calculate_new_friendship_status(acting_user, last_recorded_history, room_event)
        }
        FriendshipEvent::ACCEPT => {
            calculate_new_friendship_status(acting_user, last_recorded_history, room_event)
        }
        FriendshipEvent::CANCEL => {
            if let Some(last_history) = last_recorded_history {
                if last_history.acting_user.eq_ignore_ascii_case(acting_user) {
                    return Ok(FriendshipStatus::NotFriends);
                }
            }

            Err(FriendshipsServiceError::InternalServerError.into())
        }
        FriendshipEvent::REJECT => {
            if let Some(last_history) = last_recorded_history {
                if !last_history.acting_user.eq_ignore_ascii_case(acting_user) {
                    return Ok(FriendshipStatus::NotFriends);
                }
            }

            Err(FriendshipsServiceError::InternalServerError.into())
        }
        FriendshipEvent::DELETE => Ok(FriendshipStatus::NotFriends),
    }
}

/// Calculates the new friendship status based on the provided friendship event and the last recorded history.
/// This function assumes that the room event is valid for the last event.
fn calculate_new_friendship_status(
    acting_user: &str,
    last_recorded_history: &Option<FriendshipHistory>,
    room_event: FriendshipEvent,
) -> Result<FriendshipStatus, FriendshipsServiceErrorResponse> {
    if last_recorded_history.is_none() {
        return match room_event {
            FriendshipEvent::REQUEST => Ok(FriendshipStatus::Requested(acting_user.to_string())),
            _ => Err(FriendshipsServiceError::InternalServerError.into()),
        };
    }

    let last_history = last_recorded_history.as_ref().unwrap();

    match last_history.event {
        FriendshipEvent::REQUEST => {
            if last_history.acting_user.eq_ignore_ascii_case(acting_user) {
                return Err(FriendshipsServiceError::InternalServerError.into());
            }

            match room_event {
                FriendshipEvent::ACCEPT => Ok(FriendshipStatus::Friends),
                _ => Err(FriendshipsServiceError::InternalServerError.into()),
            }
        }
        FriendshipEvent::ACCEPT => Err(FriendshipsServiceError::InternalServerError.into()),
        _ => match room_event {
            FriendshipEvent::REQUEST => Ok(FriendshipStatus::Requested(acting_user.to_string())),
            _ => Err(FriendshipsServiceError::InternalServerError.into()),
        },
    }
}
