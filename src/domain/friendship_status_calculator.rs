use crate::{
    domain::{friendship_event::FriendshipEvent, friendship_status::FriendshipStatus, error::CommonError},
    entities::friendship_history::FriendshipHistory,
};

/// Calculates the new friendship status based on the provided friendship event and the last recorded history.
pub fn get_new_friendship_status(
    acting_user: &str,
    last_recorded_history: &Option<FriendshipHistory>,
    room_event: FriendshipEvent,
) -> Result<FriendshipStatus, CommonError> {
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
                "Get new friendship status > Invalid friendship event: {:?} for acting user: {}.",
                room_event,
                acting_user
            );
            Err(CommonError::BadRequest("Invalid friendship event update".to_owned()))
        }
        FriendshipEvent::REJECT => {
            if let Some(last_history) = last_recorded_history {
                if !last_history.acting_user.eq_ignore_ascii_case(acting_user) {
                    return Ok(FriendshipStatus::NotFriends);
                }
            }
            log::error!(
                "Get new friendship status > Invalid friendship event: {:?} for acting user: {}.",
                room_event,
                acting_user
            );
            Err(CommonError::BadRequest("Invalid friendship event update".to_owned()))
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
) -> Result<FriendshipStatus, CommonError> {
    if last_recorded_history.is_none() {
        return match room_event {
            FriendshipEvent::REQUEST => Ok(FriendshipStatus::Requested(acting_user.to_string())),
            _ => {
                log::error!(
                    "Calculate new friendship status > Invalid friendship event: {:?} for acting user: {}. Last recorded history is None, new event expected to be: {:?}.",
                    room_event,
                    acting_user,
                    FriendshipEvent::REQUEST,
                );
                Err(CommonError::BadRequest("Invalid friendship event update".to_owned()))
            }
        };
    }

    let last_history = last_recorded_history.as_ref().unwrap();

    match last_history.event {
        FriendshipEvent::REQUEST => {
            if last_history.acting_user.eq_ignore_ascii_case(acting_user) {
                log::error!(
                    "Calculate new friendship status > Invalid friendship event: {:?} for acting user: {}. Last recorded event is: {:?} and has the same acting user.",
                    room_event,
                    acting_user,
                    last_history.event
                );
                return Err(CommonError::BadRequest("Invalid friendship event update".to_owned()));
            }

            match room_event {
                FriendshipEvent::ACCEPT => Ok(FriendshipStatus::Friends),
                _ => {
                    log::error!(
                        "Calculate new friendship status > Invalid friendship event: {:?} for acting user: {}. Last recorded event is: {:?}.",
                        room_event,
                        acting_user,
                        last_history.event
                    );
                    Err(CommonError::BadRequest("Invalid friendship event update".to_owned()))
                }
            }
        }
        FriendshipEvent::ACCEPT => {
            log::error!(
                "Calculate new friendship status > Invalid friendship event: {:?} for acting user: {}. Last recorded event is: {:?}.",
                room_event,
                acting_user,
                last_history.event,
            );
            Err(CommonError::BadRequest("Invalid friendship event update".to_owned()))
        }
        _ => match room_event {
            FriendshipEvent::REQUEST => Ok(FriendshipStatus::Requested(acting_user.to_string())),
            _ => {
                log::error!(
                    "Calculate new friendship status > Invalid friendship event: {:?} for acting user: {}. Last recorded event is: {:?}.",
                    room_event,
                    acting_user,
                    last_history.event,
                );
                Err(CommonError::BadRequest("Invalid friendship event update".to_owned()
                ))
            }
        },
    }
}
