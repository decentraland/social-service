use crate::{
    domain::{error::CommonError, friendship_event::FriendshipEvent},
    entities::friendship_history::FriendshipHistory,
};

/**
* Validates that the transition between current state and new event is valid and the acting user is authorized to perform the action
*/
pub fn validate_new_event(
    acting_user: &str,
    last_recorded_history: &Option<FriendshipHistory>,
    new_event: FriendshipEvent,
) -> Result<(), CommonError> {
    validate_transition(last_recorded_history, new_event)?;
    validate_auth_new_event(acting_user, last_recorded_history, new_event)?;
    Ok(())
}

/**
 * Validates that the new event is valid for the current state in last_recorded_history
 */
fn validate_transition(
    last_recorded_history: &Option<FriendshipHistory>,
    new_event: FriendshipEvent,
) -> Result<(), CommonError> {
    let last_recorded_event = last_recorded_history.as_ref().map(|history| history.event);
    let is_valid = FriendshipEvent::validate_new_event_is_valid(&last_recorded_event, new_event);
    if !is_valid {
        log::error!(
            "Validate transition > Invalid friendship event: {:?}. The last recorded event is: {:?}.",
            new_event,
            last_recorded_event
        );
        return Err(CommonError::BadRequest(
            "Invalid friendship event update transition".to_owned(),
        ));
    };
    Ok(())
}

/**
* Returns an error if the acting user is invalid for the action, this method assumes that the transition is valid
*/
fn validate_auth_new_event(
    acting_user: &str,
    last_recorded_history: &Option<FriendshipHistory>,
    new_event: FriendshipEvent,
) -> Result<(), CommonError> {
    if let Some(last_history) = last_recorded_history {
        if last_history.acting_user.eq_ignore_ascii_case(acting_user) {
            match new_event {
                FriendshipEvent::ACCEPT => {
                    log::error!(
                        "Validate auth new event > Invalid acting user for friendship event: {:?}. The last recorded event is: {:?}.",
                        new_event,
                        last_history.event
                    );
                    Err(CommonError::BadRequest(
                        "Invalid acting user for friendship event update accept".to_owned(),
                    ))
                }
                FriendshipEvent::REJECT => {
                    log::error!(
                        "Validate auth new event > Invalid acting user for friendship event: {:?}. The last recorded event is: {:?}.",
                        new_event,
                        last_history.event
                    );
                    Err(CommonError::BadRequest(
                        "Invalid acting user for friendship event update reject".to_owned(),
                    ))
                }
                _ => Ok(()),
            }
        } else {
            match new_event {
                FriendshipEvent::CANCEL => {
                    log::error!(
                        "Validate auth new event > Invalid acting user for friendship event: {:?}. The last recorded event is: {:?}.",
                        new_event,
                        last_history.event
                    );
                    Err(CommonError::BadRequest(
                        "Invalid acting user for friendship event update cancel".to_owned(),
                    ))
                }
                _ => Ok(()),
            }
        }
    } else {
        Ok(())
    }
}
