use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    entities::friendship_history::FriendshipRequestEvent,
    friendships::friendship_event_payload,
    friendships::friendship_event_response,
    friendships::AcceptResponse,
    friendships::CancelResponse,
    friendships::DeleteResponse,
    friendships::FriendshipEventResponse,
    friendships::RejectResponse,
    friendships::RequestEvents,
    friendships::RequestResponse,
    friendships::UpdateFriendshipPayload,
    friendships::UpdateFriendshipResponse,
    friendships::User,
    friendships::{FriendshipEventPayload, Requests},
    models::friendship_event::FriendshipEvent,
    notifications::Event,
    ws::service::{
        errors::{FriendshipsServiceError, FriendshipsServiceErrorResponse},
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

pub fn event_response_as_update_response(
    request: UpdateFriendshipPayload,
    result: EventResponse,
) -> Result<UpdateFriendshipResponse, FriendshipsServiceErrorResponse> {
    let event_response = if let Some(body) = request.event {
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
            None => return Err(FriendshipsServiceError::InternalServerError.into()),
        }
    } else {
        return Err(FriendshipsServiceError::InternalServerError.into());
    };

    Ok(event_response)
}

pub fn update_friendship_payload_as_event(
    payload: FriendshipEventPayload,
    from: &str,
    created_at: i64,
) -> Option<Event> {
    if let Ok((friendship_event, to)) = payload_event_as_response(payload, from, created_at) {
        Some(Event {
            friendship_event,
            from: from.to_string(),
            to,
        })
    } else {
        None
    }
}

fn payload_event_as_response(
    payload: FriendshipEventPayload,
    from: &str,
    created_at: i64,
) -> Result<(Option<FriendshipEventResponse>, String), ()> {
    match payload.body {
        Some(friendship_event_payload::Body::Request(request)) => {
            request_payload_as_response(request, from, created_at)
        }
        Some(friendship_event_payload::Body::Accept(accept)) => {
            accept_payload_as_response(accept, from)
        }
        Some(friendship_event_payload::Body::Reject(reject)) => {
            reject_payload_as_response(reject, from)
        }
        Some(friendship_event_payload::Body::Cancel(cancel)) => {
            cancel_payload_as_response(cancel, from)
        }
        Some(friendship_event_payload::Body::Delete(delete)) => {
            delete_payload_as_response(delete, from)
        }
        None => Err(()),
    }
}

fn user(from: &str) -> Option<User> {
    Some(User {
        address: from.to_string(),
    })
}

fn request_payload_as_response(
    request: crate::friendships::RequestPayload,
    from: &str,
    created_at: i64,
) -> Result<(Option<FriendshipEventResponse>, String), ()> {
    if let Some(user_to) = request.user.map(|u| u.address) {
        let request = friendship_event_response::Body::Request(RequestResponse {
            user: user(from),
            created_at,
            message: request.message,
        });
        let event = FriendshipEventResponse {
            body: Some(request),
        };
        Ok((Some(event), user_to))
    } else {
        Err(())
    }
}

fn accept_payload_as_response(
    accept: crate::friendships::AcceptPayload,
    from: &str,
) -> Result<(Option<FriendshipEventResponse>, String), ()> {
    if let Some(user_to) = accept.user.map(|u| u.address) {
        let accept = friendship_event_response::Body::Accept(AcceptResponse { user: user(from) });
        let event = FriendshipEventResponse { body: Some(accept) };
        Ok((Some(event), user_to))
    } else {
        Err(())
    }
}

fn reject_payload_as_response(
    reject: crate::friendships::RejectPayload,
    from: &str,
) -> Result<(Option<FriendshipEventResponse>, String), ()> {
    if let Some(user_to) = reject.user.map(|u| u.address) {
        let reject = friendship_event_response::Body::Reject(RejectResponse { user: user(from) });
        let event = FriendshipEventResponse { body: Some(reject) };
        Ok((Some(event), user_to))
    } else {
        Err(())
    }
}

fn cancel_payload_as_response(
    cancel: crate::friendships::CancelPayload,
    from: &str,
) -> Result<(Option<FriendshipEventResponse>, String), ()> {
    if let Some(user_to) = cancel.user.map(|u| u.address) {
        let cancel = friendship_event_response::Body::Cancel(CancelResponse { user: user(from) });
        let event = FriendshipEventResponse { body: Some(cancel) };
        Ok((Some(event), user_to))
    } else {
        Err(())
    }
}

fn delete_payload_as_response(
    delete: crate::friendships::DeletePayload,
    from: &str,
) -> Result<(Option<FriendshipEventResponse>, String), ()> {
    if let Some(user_to) = delete.user.map(|u| u.address) {
        let delete = friendship_event_response::Body::Delete(DeleteResponse { user: user(from) });
        let event = FriendshipEventResponse { body: Some(delete) };
        Ok((Some(event), user_to))
    } else {
        Err(())
    }
}

#[test]
fn test_request_as_response() {
    let payload = FriendshipEventPayload {
        body: Some(friendship_event_payload::Body::Request(
            crate::friendships::RequestPayload {
                user: Some(User {
                    address: "0xBob".to_owned(),
                }),
                message: Some("Hi Bob, let's be friends!".to_owned()),
            },
        )),
    };
    let result = payload_event_as_response(payload, "0xAlice", 1234567890);
    match result {
        Ok((response, user_to)) => {
            assert_eq!(
                response,
                Some(FriendshipEventResponse {
                    body: Some(friendship_event_response::Body::Request(RequestResponse {
                        user: Some(User {
                            address: "0xAlice".to_owned(),
                        }),
                        created_at: 1234567890,
                        message: Some("Hi Bob, let's be friends!".to_owned()),
                    })),
                })
            );
            assert_eq!(user_to, "0xBob");
        }
        Err(()) => {
            unreachable!("Parsing to request response should not fail")
        }
    }
}

#[test]
fn test_accept_as_response() {
    let payload = FriendshipEventPayload {
        body: Some(friendship_event_payload::Body::Accept(
            crate::friendships::AcceptPayload {
                user: Some(User {
                    address: "0xBob".to_owned(),
                }),
            },
        )),
    };
    let result = payload_event_as_response(payload, "0xAlice", 1234567890);
    match result {
        Ok((response, user_to)) => {
            assert_eq!(
                response,
                Some(FriendshipEventResponse {
                    body: Some(friendship_event_response::Body::Accept(AcceptResponse {
                        user: Some(User {
                            address: "0xAlice".to_owned(),
                        })
                    })),
                })
            );
            assert_eq!(user_to, "0xBob");
        }
        Err(()) => {
            unreachable!("Parsing to accept response should not fail")
        }
    }
}

#[test]
fn test_reject_as_response() {
    let payload = FriendshipEventPayload {
        body: Some(friendship_event_payload::Body::Reject(
            crate::friendships::RejectPayload {
                user: Some(User {
                    address: "0xBob".to_owned(),
                }),
            },
        )),
    };
    let result = payload_event_as_response(payload, "0xAlice", 1234567890);
    match result {
        Ok((response, user_to)) => {
            assert_eq!(
                response,
                Some(FriendshipEventResponse {
                    body: Some(friendship_event_response::Body::Reject(RejectResponse {
                        user: Some(User {
                            address: "0xAlice".to_owned(),
                        })
                    })),
                })
            );
            assert_eq!(user_to, "0xBob");
        }
        Err(()) => {
            unreachable!("Parsing to reject response should not fail")
        }
    }
}

#[test]
fn test_cancel_as_response() {
    let payload = FriendshipEventPayload {
        body: Some(friendship_event_payload::Body::Cancel(
            crate::friendships::CancelPayload {
                user: Some(User {
                    address: "0xBob".to_owned(),
                }),
            },
        )),
    };
    let result = payload_event_as_response(payload, "0xAlice", 1234567890);
    match result {
        Ok((response, user_to)) => {
            assert_eq!(
                response,
                Some(FriendshipEventResponse {
                    body: Some(friendship_event_response::Body::Cancel(CancelResponse {
                        user: Some(User {
                            address: "0xAlice".to_owned(),
                        })
                    })),
                })
            );
            assert_eq!(user_to, "0xBob");
        }
        Err(()) => {
            unreachable!("Parsing to cancel response should not fail")
        }
    }
}

#[test]
fn test_delete_as_response() {
    let payload = FriendshipEventPayload {
        body: Some(friendship_event_payload::Body::Delete(
            crate::friendships::DeletePayload {
                user: Some(User {
                    address: "0xBob".to_owned(),
                }),
            },
        )),
    };
    let result = payload_event_as_response(payload, "0xAlice", 1234567890);
    match result {
        Ok((response, user_to)) => {
            assert_eq!(
                response,
                Some(FriendshipEventResponse {
                    body: Some(friendship_event_response::Body::Delete(DeleteResponse {
                        user: Some(User {
                            address: "0xAlice".to_owned(),
                        })
                    })),
                })
            );
            assert_eq!(user_to, "0xBob");
        }
        Err(()) => {
            unreachable!("Parsing to delete response should not fail")
        }
    }
}
