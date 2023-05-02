use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    api::routes::v1::error::CommonError,
    entities::friendship_history::FriendshipRequestEvent,
    friendships::{
        friendship_event_payload, friendship_event_response, AcceptResponse, CancelResponse,
        DeleteResponse, FriendshipEventResponse, RejectResponse, RequestEvents, RequestResponse,
        Requests, UpdateFriendshipPayload, UpdateFriendshipResponse, User,
    },
    models::friendship_event::FriendshipEvent,
    ws::service::{
        errors::FriendshipsServiceError,
        types::{EventPayload, EventResponse},
    },
};

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
            // If the acting user is the same as the user id, then the request is outgoing
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
pub fn update_request_as_event_payload(
    request: UpdateFriendshipPayload,
) -> Result<EventPayload, FriendshipsServiceError> {
    let event_payload = if let Some(body) = request.event {
        match body.body {
            Some(friendship_event_payload::Body::Request(request)) => EventPayload {
                friendship_event: FriendshipEvent::REQUEST,
                request_event_message_body: request.message,
                second_user: request
                    .user
                    .ok_or(FriendshipsServiceError::BadRequest(
                        "`user address` is missing".to_string(),
                    ))?
                    .address,
            },
            Some(friendship_event_payload::Body::Accept(accept)) => EventPayload {
                friendship_event: FriendshipEvent::ACCEPT,
                request_event_message_body: None,
                second_user: accept
                    .user
                    .ok_or(FriendshipsServiceError::BadRequest(
                        "`user address` is missing".to_string(),
                    ))?
                    .address,
            },
            Some(friendship_event_payload::Body::Reject(reject)) => EventPayload {
                friendship_event: FriendshipEvent::REJECT,
                request_event_message_body: None,
                second_user: reject
                    .user
                    .ok_or(FriendshipsServiceError::BadRequest(
                        "`user address` is missing".to_string(),
                    ))?
                    .address,
            },
            Some(friendship_event_payload::Body::Cancel(cancel)) => EventPayload {
                friendship_event: FriendshipEvent::CANCEL,
                request_event_message_body: None,
                second_user: cancel
                    .user
                    .ok_or(FriendshipsServiceError::BadRequest(
                        "`user address` is missing".to_string(),
                    ))?
                    .address,
            },
            Some(friendship_event_payload::Body::Delete(delete)) => EventPayload {
                friendship_event: FriendshipEvent::DELETE,
                request_event_message_body: None,
                second_user: delete
                    .user
                    .ok_or(FriendshipsServiceError::BadRequest(
                        "`user address` is missing".to_string(),
                    ))?
                    .address,
            },
            None => {
                return Err(FriendshipsServiceError::BadRequest(
                    "`friendship_event_payload::body` is missing".to_string(),
                ))
            }
        }
    } else {
        return Err(FriendshipsServiceError::BadRequest(
            "`event` is missing".to_string(),
        ));
    };

    Ok(event_payload)
}

pub fn event_response_as_update_response(
    request: UpdateFriendshipPayload,
    result: EventResponse,
) -> Result<UpdateFriendshipResponse, FriendshipsServiceError> {
    let update_response = if let Some(body) = request.event {
        match body.body {
            Some(friendship_event_payload::Body::Request(payload)) => {
                let request_response = RequestResponse {
                    user: Some(User {
                        address: result.user_id,
                    }),
                    created_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    message: payload.message,
                };

                let body = friendship_event_response::Body::Request(request_response);
                let event: FriendshipEventResponse = FriendshipEventResponse { body: Some(body) };

                UpdateFriendshipResponse { event: Some(event) }
            }
            Some(friendship_event_payload::Body::Accept(_)) => {
                let accept_response = AcceptResponse {
                    user: Some(User {
                        address: result.user_id,
                    }),
                };

                let body = friendship_event_response::Body::Accept(accept_response);
                let event: FriendshipEventResponse = FriendshipEventResponse { body: Some(body) };

                UpdateFriendshipResponse { event: Some(event) }
            }
            Some(friendship_event_payload::Body::Reject(_)) => {
                let reject_response = RejectResponse {
                    user: Some(User {
                        address: result.user_id,
                    }),
                };

                let body = friendship_event_response::Body::Reject(reject_response);
                let event: FriendshipEventResponse = FriendshipEventResponse { body: Some(body) };

                UpdateFriendshipResponse { event: Some(event) }
            }
            Some(friendship_event_payload::Body::Cancel(_)) => {
                let cancel_response = CancelResponse {
                    user: Some(User {
                        address: result.user_id,
                    }),
                };

                let body = friendship_event_response::Body::Cancel(cancel_response);
                let event: FriendshipEventResponse = FriendshipEventResponse { body: Some(body) };

                UpdateFriendshipResponse { event: Some(event) }
            }
            Some(friendship_event_payload::Body::Delete(_)) => {
                let delete_response = DeleteResponse {
                    user: Some(User {
                        address: result.user_id,
                    }),
                };

                let body = friendship_event_response::Body::Delete(delete_response);
                let event: FriendshipEventResponse = FriendshipEventResponse { body: Some(body) };

                UpdateFriendshipResponse { event: Some(event) }
            }
            None => return Err(FriendshipsServiceError::InternalServerError),
        }
    } else {
        return Err(FriendshipsServiceError::InternalServerError);
    };

    Ok(update_response)
}

pub fn map_common_error_to_friendships_error(err: CommonError) -> FriendshipsServiceError {
    match err {
        CommonError::Forbidden(_) => FriendshipsServiceError::Forbidden("".to_owned()),
        CommonError::Unauthorized => FriendshipsServiceError::Unauthorized("".to_owned()),
        CommonError::TooManyRequests => FriendshipsServiceError::TooManyRequests("".to_owned()),
        _ => FriendshipsServiceError::InternalServerError,
    }
}

#[cfg(test)]
mod tests {
    use super::map_common_error_to_friendships_error;
    use crate::{
        api::routes::v1::error::CommonError, ws::service::errors::FriendshipsServiceError,
    };

    #[test]
    fn test_map_common_error_to_friendships_error() {
        let err = CommonError::Forbidden("".to_owned());
        assert_eq!(
            map_common_error_to_friendships_error(err),
            FriendshipsServiceError::Forbidden("".to_owned())
        );

        let err = CommonError::Unauthorized;
        assert_eq!(
            map_common_error_to_friendships_error(err),
            FriendshipsServiceError::Unauthorized("".to_owned())
        );

        let err = CommonError::TooManyRequests;
        assert_eq!(
            map_common_error_to_friendships_error(err),
            FriendshipsServiceError::TooManyRequests("".to_owned())
        );

        let err = CommonError::Unknown;
        assert_eq!(
            map_common_error_to_friendships_error(err),
            FriendshipsServiceError::InternalServerError
        );
    }
}
