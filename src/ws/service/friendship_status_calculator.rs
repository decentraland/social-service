use crate::{
    entities::friendship_history::FriendshipHistory,
    models::{friendship_event::FriendshipEvent, friendship_status::FriendshipStatus},
    ws::service::errors::FriendshipsServiceError,
};

/// Calculates the new friendship status based on the provided friendship event and the last recorded history.
pub fn get_new_friendship_status(
    acting_user: &str,
    last_recorded_history: &Option<FriendshipHistory>,
    room_event: FriendshipEvent,
) -> Result<FriendshipStatus, FriendshipsServiceError> {
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
            log::error!(
                "Get new friendship status > Invalid friendship event: {:?} for acting user: {}",
                room_event,
                acting_user
            );
            Err(FriendshipsServiceError::BadRequest(
                "Invalid friendship event update".to_string(),
            ))
        }
        FriendshipEvent::REJECT => {
            if let Some(last_history) = last_recorded_history {
                if !last_history.acting_user.eq_ignore_ascii_case(acting_user) {
                    return Ok(FriendshipStatus::NotFriends);
                }
            }
            log::error!(
                "Get new friendship status > Invalid friendship event: {:?} for acting user: {}",
                room_event,
                acting_user
            );
            Err(FriendshipsServiceError::BadRequest(
                "Invalid friendship event update".to_string(),
            ))
        }
        FriendshipEvent::DELETE => Ok(FriendshipStatus::NotFriends),
    }
}

/// Calculates the new friendship status based on the provided friendship event and the last recorded history.
/// Assumes that the room event is valid for the last event.
fn calculate_new_friendship_status(
    acting_user: &str,
    last_recorded_history: &Option<FriendshipHistory>,
    room_event: FriendshipEvent,
) -> Result<FriendshipStatus, FriendshipsServiceError> {
    if last_recorded_history.is_none() {
        return match room_event {
            FriendshipEvent::REQUEST => Ok(FriendshipStatus::Requested(acting_user.to_string())),
            _ => {
                log::error!(
                    "Calculate new friendship status > Invalid friendship event: {:?} for acting user: {}. Last recorded history is none, new event expected to be: {:?}",
                    room_event,
                    acting_user,
                    FriendshipEvent::REQUEST,
                );
                Err(FriendshipsServiceError::BadRequest(
                    "Invalid friendship event update".to_string(),
                ))
            }
        };
    }

    let last_history = last_recorded_history.as_ref().unwrap();

    match last_history.event {
        FriendshipEvent::REQUEST => {
            if last_history.acting_user.eq_ignore_ascii_case(acting_user) {
                log::error!(
                    "Calculate new friendship status > Invalid friendship event: {:?} for acting user: {}. Last recorded event is: {:?} and has the same acting user",
                    room_event,
                    acting_user,
                    last_history.event
                );
                return Err(FriendshipsServiceError::BadRequest(
                    "Invalid friendship event update".to_string(),
                ));
            }

            match room_event {
                FriendshipEvent::ACCEPT => Ok(FriendshipStatus::Friends),
                _ => {
                    log::error!(
                        "Calculate new friendship status > Invalid friendship event: {:?} for acting user: {}. Last recorded event is: {:?}",
                        room_event,
                        acting_user,
                        last_history.event
                    );
                    Err(FriendshipsServiceError::BadRequest(
                        "Invalid friendship event update".to_string(),
                    ))
                }
            }
        }
        FriendshipEvent::ACCEPT => {
            log::error!(
                "Calculate new friendship status > Invalid friendship event: {:?} for acting user: {}. Last recorded event is: {:?}",
                room_event,
                acting_user,
                last_history.event,
            );
            Err(FriendshipsServiceError::BadRequest(
                "Invalid friendship event update".to_string(),
            ))
        }
        _ => match room_event {
            FriendshipEvent::REQUEST => Ok(FriendshipStatus::Requested(acting_user.to_string())),
            _ => {
                log::error!(
                    "Calculate new friendship status > Invalid friendship event: {:?} for acting user: {}. Last recorded event is: {:?}",
                    room_event,
                    acting_user,
                    last_history.event,
                );
                Err(FriendshipsServiceError::BadRequest(
                    "Invalid friendship event update".to_string(),
                ))
            }
        },
    }
}
