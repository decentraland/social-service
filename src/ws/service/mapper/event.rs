use crate::{
    domain::{
        error::CommonError,
        event::{EventPayload, EventResponse},
        friendship_event::FriendshipEvent,
    },
    entities::friendship_history::FriendshipRequestEvent,
    friendships::{
        friendship_event_payload, friendship_event_response, request_events_response,
        update_friendship_response, AcceptResponse, CancelResponse, DeleteResponse,
        FriendshipEventPayload, FriendshipEventResponse, RejectResponse, RequestEvents,
        RequestEventsResponse, RequestResponse, Requests, UpdateFriendshipPayload,
        UpdateFriendshipResponse, User,
    },
    notifications::Event,
};

use super::response::payload_event_as_response;

/// Maps a list of `FriendshipRequestEvents` to a `RequestEvents` struct.
///
/// * `requests` - A vector of `FriendshipRequestEvents` to map to `RequestResponse` struct.
/// * `user_id` - The id of the auth user.
pub fn friendship_requests_as_request_events_response(
    requests: Vec<FriendshipRequestEvent>,
    user_id: String,
) -> RequestEventsResponse {
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
    RequestEventsResponse::from_response(request_events_response::Response::Events(RequestEvents {
        outgoing: Some(Requests {
            total: outgoing_requests.len() as i64,
            items: outgoing_requests,
        }),
        incoming: Some(Requests {
            total: incoming_requests.len() as i64,
            items: incoming_requests,
        }),
    }))
}

/// Extracts the information from a friendship update payload,
/// that is, the room event, the other user who is part of the friendship event, and the message body from the request event.
pub fn update_request_as_event_payload(
    request: UpdateFriendshipPayload,
) -> Result<EventPayload, CommonError> {
    let event_payload = if let Some(body) = request.event {
        match body.body {
            Some(friendship_event_payload::Body::Request(request)) => EventPayload {
                friendship_event: FriendshipEvent::REQUEST,
                request_event_message_body: request.message,
                second_user: request
                    .user
                    .ok_or(CommonError::BadRequest(
                        "`user address` is missing".to_owned(),
                    ))?
                    .address,
            },
            Some(friendship_event_payload::Body::Accept(accept)) => EventPayload {
                friendship_event: FriendshipEvent::ACCEPT,
                request_event_message_body: None,
                second_user: accept
                    .user
                    .ok_or(CommonError::BadRequest(
                        "`user address` is missing".to_owned(),
                    ))?
                    .address,
            },
            Some(friendship_event_payload::Body::Reject(reject)) => EventPayload {
                friendship_event: FriendshipEvent::REJECT,
                request_event_message_body: None,
                second_user: reject
                    .user
                    .ok_or(CommonError::BadRequest(
                        "`user address` is missing".to_owned(),
                    ))?
                    .address,
            },
            Some(friendship_event_payload::Body::Cancel(cancel)) => EventPayload {
                friendship_event: FriendshipEvent::CANCEL,
                request_event_message_body: None,
                second_user: cancel
                    .user
                    .ok_or(CommonError::BadRequest(
                        "`user address` is missing".to_owned(),
                    ))?
                    .address,
            },
            Some(friendship_event_payload::Body::Delete(delete)) => EventPayload {
                friendship_event: FriendshipEvent::DELETE,
                request_event_message_body: None,
                second_user: delete
                    .user
                    .ok_or(CommonError::BadRequest(
                        "`user address` is missing".to_owned(),
                    ))?
                    .address,
            },
            None => {
                return Err(CommonError::BadRequest(
                    "`friendship_event_payload::body` is missing".to_owned(),
                ))
            }
        }
    } else {
        return Err(CommonError::BadRequest("`event` is missing".to_owned()));
    };

    Ok(event_payload)
}

///
pub fn event_response_as_update_response(
    request: UpdateFriendshipPayload,
    result: EventResponse,
    created_at: i64,
) -> Result<UpdateFriendshipResponse, CommonError> {
    let update_response = if let Some(body) = request.event {
        match body.body {
            Some(friendship_event_payload::Body::Request(payload)) => {
                let request_response = RequestResponse {
                    user: Some(User {
                        address: result.user_id,
                    }),
                    created_at,
                    message: payload.message,
                };

                let body = friendship_event_response::Body::Request(request_response);
                let event: FriendshipEventResponse = FriendshipEventResponse { body: Some(body) };

                UpdateFriendshipResponse::from_response(
                    update_friendship_response::Response::Event(event),
                )
            }
            Some(friendship_event_payload::Body::Accept(_)) => {
                let accept_response = AcceptResponse {
                    user: Some(User {
                        address: result.user_id,
                    }),
                };

                let body = friendship_event_response::Body::Accept(accept_response);
                let event: FriendshipEventResponse = FriendshipEventResponse { body: Some(body) };

                UpdateFriendshipResponse::from_response(
                    update_friendship_response::Response::Event(event),
                )
            }
            Some(friendship_event_payload::Body::Reject(_)) => {
                let reject_response = RejectResponse {
                    user: Some(User {
                        address: result.user_id,
                    }),
                };

                let body = friendship_event_response::Body::Reject(reject_response);
                let event: FriendshipEventResponse = FriendshipEventResponse { body: Some(body) };

                UpdateFriendshipResponse::from_response(
                    update_friendship_response::Response::Event(event),
                )
            }
            Some(friendship_event_payload::Body::Cancel(_)) => {
                let cancel_response = CancelResponse {
                    user: Some(User {
                        address: result.user_id,
                    }),
                };

                let body = friendship_event_response::Body::Cancel(cancel_response);
                let event: FriendshipEventResponse = FriendshipEventResponse { body: Some(body) };

                UpdateFriendshipResponse::from_response(
                    update_friendship_response::Response::Event(event),
                )
            }
            Some(friendship_event_payload::Body::Delete(_)) => {
                let delete_response = DeleteResponse {
                    user: Some(User {
                        address: result.user_id,
                    }),
                };

                let body = friendship_event_response::Body::Delete(delete_response);
                let event: FriendshipEventResponse = FriendshipEventResponse { body: Some(body) };

                UpdateFriendshipResponse::from_response(
                    update_friendship_response::Response::Event(event),
                )
            }
            None => return Err(CommonError::Unknown("Unexpected error".to_owned())),
        }
    } else {
        return Err(CommonError::Unknown("Unexpected error".to_owned()));
    };

    Ok(update_response)
}

/// Maps a `FriendshipEventPayload` to an `Event` struct.
pub fn update_friendship_payload_as_event(
    payload: FriendshipEventPayload,
    from: &str,
    created_at: i64,
) -> Result<Event, CommonError> {
    if let Ok((friendship_event, to)) = payload_event_as_response(payload, from, created_at) {
        Ok(Event {
            friendship_event: Some(friendship_event),
            from: from.to_string(),
            to,
        })
    } else {
        Err(CommonError::Unknown("".to_owned()))
    }
}

/// Maps a `FriendshipEventPayload` to an `FriendshipEvent` struct.
pub fn parse_event_payload_to_friendship_event(
    payload: FriendshipEventPayload,
) -> Option<FriendshipEvent> {
    match payload.body? {
        friendship_event_payload::Body::Request(_) => Some(FriendshipEvent::REQUEST),
        friendship_event_payload::Body::Accept(_) => Some(FriendshipEvent::ACCEPT),
        friendship_event_payload::Body::Reject(_) => Some(FriendshipEvent::REJECT),
        friendship_event_payload::Body::Delete(_) => Some(FriendshipEvent::DELETE),
        friendship_event_payload::Body::Cancel(_) => Some(FriendshipEvent::CANCEL),
    }
}
