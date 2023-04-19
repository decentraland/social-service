use crate::{
    entities::{
        friendship_event::FriendshipEvent,
        friendship_history::{FriendshipHistory, FriendshipRequestEvent},
        friendship_status::FriendshipStatus,
    },
    friendship_event_payload,
    ws::service::errors::FriendshipsServiceError,
    ws::service::errors::FriendshipsServiceErrorResponse,
    RequestEvents, RequestResponse, Requests, UpdateFriendshipPayload, User,
};

use super::types::EventPayload;

/// Maps a list of `FriendshipRequestEvents` to a `RequestEvents` struct.
///
/// * `requests` - A vector of `FriendshipRequestEvents` to map to `RequestResponse` struct.
/// * `user_id` - The id of the auth user.
pub fn friendship_requests_as_request_events(
    requests: Vec<FriendshipRequestEvent>,
    user_id: String,
) -> RequestEvents {
    let mut outgoing_requests: Vec<RequestResponse> = Vec::new();
    let mut incoming_requests: Vec<RequestResponse> = Vec::new();

    // Iterate through each friendship request event
    for request in requests {
        // Get the user id of the acting user for the request
        let acting_user_id = request.acting_user.clone();

        // Determine the address of the other user involved in the request event
        let address = if request.address_1.eq_ignore_ascii_case(&user_id) {
            request.address_2.clone()
        } else {
            request.address_1.clone()
        };

        // Get the message (if any) associated with the request
        let message = request
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.message.clone());

        let request_response = RequestResponse {
            user: Some(User { address }),
            created_at: request.timestamp.timestamp(),
            message,
        };

        if acting_user_id.eq_ignore_ascii_case(&user_id) {
            // If the acting user is the same as the user ID, then the request is outgoing
            outgoing_requests.push(request_response);
        } else {
            // Otherwise, the request is incoming
            incoming_requests.push(request_response);
        }
    }

    // Return a RequestEvents struct containing the incoming and outgoing request lists
    RequestEvents {
        outgoing: Some(Requests {
            total: outgoing_requests.len() as i64,
            items: outgoing_requests,
        }),
        incoming: Some(Requests {
            total: incoming_requests.len() as i64,
            items: incoming_requests,
        }),
    }
}

/// Extracts the information from a friendship update payload,
/// that is, the room event, the other user who is part of the friendship event, and the message body from the request event.
pub fn extract_update_friendship_payload(
    request: UpdateFriendshipPayload,
) -> Result<EventPayload, FriendshipsServiceErrorResponse> {
    let event_payload = if let Some(body) = request.event {
        match body.body {
            Some(friendship_event_payload::Body::Request(request)) => EventPayload {
                friendship_event: FriendshipEvent::REQUEST,
                request_event_message_body: request.message,
                second_user: request
                    .user
                    .ok_or(FriendshipsServiceError::InternalServerError)?
                    .address,
            },
            Some(friendship_event_payload::Body::Accept(accept)) => EventPayload {
                friendship_event: FriendshipEvent::ACCEPT,
                request_event_message_body: None,
                second_user: accept
                    .user
                    .ok_or(FriendshipsServiceError::InternalServerError)?
                    .address,
            },
            Some(friendship_event_payload::Body::Reject(reject)) => EventPayload {
                friendship_event: FriendshipEvent::REJECT,
                request_event_message_body: None,
                second_user: reject
                    .user
                    .ok_or(FriendshipsServiceError::InternalServerError)?
                    .address,
            },
            Some(friendship_event_payload::Body::Cancel(cancel)) => EventPayload {
                friendship_event: FriendshipEvent::CANCEL,
                request_event_message_body: None,
                second_user: cancel
                    .user
                    .ok_or(FriendshipsServiceError::InternalServerError)?
                    .address,
            },
            Some(friendship_event_payload::Body::Delete(delete)) => EventPayload {
                friendship_event: FriendshipEvent::DELETE,
                request_event_message_body: None,
                second_user: delete
                    .user
                    .ok_or(FriendshipsServiceError::InternalServerError)?
                    .address,
            },
            None => return Err(FriendshipsServiceError::InternalServerError.into()),
        }
    } else {
        return Err(FriendshipsServiceError::InternalServerError.into());
    };

    Ok(event_payload)
}

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

/// Builds a room alias name from a vector of user addresses by sorting them and joining them with a "+" separator.
///
/// * `user_ids` - A mut vector of users addresses as strings.
///
/// Returns the room alias name as a string.
pub fn build_room_alias_name(mut user_ids: Vec<&str>) -> String {
    user_ids.sort();
    user_ids.join("+")
}

/// Validates the new event is valid and different from the last recorded.
pub fn validate_new_event(
    last_recorded_history: &Option<FriendshipHistory>,
    new_event: FriendshipEvent,
) -> Result<(), FriendshipsServiceErrorResponse> {
    let last_recorded_event = last_recorded_history.as_ref().map(|history| history.event);
    let is_valid = FriendshipEvent::validate_new_event_is_valid(&last_recorded_event, new_event);
    if !is_valid {
        return Err(FriendshipsServiceError::InternalServerError.into());
    };
    Ok(())
}
