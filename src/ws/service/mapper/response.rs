use crate::{
    domain::error::CommonError,
    friendships::{
        friendship_event_payload, friendship_event_response, request_events_response,
        subscribe_friendship_events_updates_response, update_friendship_response, users_response,
        AcceptResponse, CancelResponse, DeleteResponse, FriendshipEventPayload,
        FriendshipEventResponse, RejectResponse, RequestEventsResponse, RequestResponse,
        SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipResponse, User, UsersResponse,
    },
};

impl UsersResponse {
    pub fn from_response(response: users_response::Response) -> Self {
        Self {
            response: Some(response),
        }
    }
}
impl RequestEventsResponse {
    pub fn from_response(response: request_events_response::Response) -> Self {
        Self {
            response: Some(response),
        }
    }
}
impl UpdateFriendshipResponse {
    pub fn from_response(response: update_friendship_response::Response) -> Self {
        Self {
            response: Some(response),
        }
    }
}
impl SubscribeFriendshipEventsUpdatesResponse {
    pub fn from_response(response: subscribe_friendship_events_updates_response::Response) -> Self {
        Self {
            response: Some(response),
        }
    }
}

pub fn payload_event_as_response(
    payload: FriendshipEventPayload,
    from: &str,
    created_at: i64,
) -> Result<(FriendshipEventResponse, String), CommonError> {
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
        None => Err(CommonError::Unknown("".to_owned())),
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
) -> Result<(FriendshipEventResponse, String), CommonError> {
    match request.user.map(|u| u.address) {
        Some(user_to) => {
            let request = friendship_event_response::Body::Request(RequestResponse {
                user: user(from),
                created_at,
                message: request.message,
            });
            let event = FriendshipEventResponse {
                body: Some(request),
            };
            Ok((event, user_to))
        }
        None => Err(CommonError::Unknown("".to_owned())),
    }
}

fn accept_payload_as_response(
    accept: crate::friendships::AcceptPayload,
    from: &str,
) -> Result<(FriendshipEventResponse, String), CommonError> {
    if let Some(user_to) = accept.user.map(|u| u.address) {
        let accept = friendship_event_response::Body::Accept(AcceptResponse { user: user(from) });
        let event = FriendshipEventResponse { body: Some(accept) };
        Ok((event, user_to))
    } else {
        Err(CommonError::Unknown("".to_owned()))
    }
}

fn reject_payload_as_response(
    reject: crate::friendships::RejectPayload,
    from: &str,
) -> Result<(FriendshipEventResponse, String), CommonError> {
    if let Some(user_to) = reject.user.map(|u| u.address) {
        let reject = friendship_event_response::Body::Reject(RejectResponse { user: user(from) });
        let event = FriendshipEventResponse { body: Some(reject) };
        Ok((event, user_to))
    } else {
        Err(CommonError::Unknown("".to_owned()))
    }
}

fn cancel_payload_as_response(
    cancel: crate::friendships::CancelPayload,
    from: &str,
) -> Result<(FriendshipEventResponse, String), CommonError> {
    if let Some(user_to) = cancel.user.map(|u| u.address) {
        let cancel = friendship_event_response::Body::Cancel(CancelResponse { user: user(from) });
        let event = FriendshipEventResponse { body: Some(cancel) };
        Ok((event, user_to))
    } else {
        Err(CommonError::Unknown("".to_owned()))
    }
}

fn delete_payload_as_response(
    delete: crate::friendships::DeletePayload,
    from: &str,
) -> Result<(FriendshipEventResponse, String), CommonError> {
    if let Some(user_to) = delete.user.map(|u| u.address) {
        let delete = friendship_event_response::Body::Delete(DeleteResponse { user: user(from) });
        let event = FriendshipEventResponse { body: Some(delete) };
        Ok((event, user_to))
    } else {
        Err(CommonError::Unknown("".to_owned()))
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
                FriendshipEventResponse {
                    body: Some(friendship_event_response::Body::Request(RequestResponse {
                        user: Some(User {
                            address: "0xAlice".to_owned(),
                        }),
                        created_at: 1234567890,
                        message: Some("Hi Bob, let's be friends!".to_owned()),
                    })),
                }
            );
            assert_eq!(user_to, "0xBob");
        }
        Err(_) => {
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
                FriendshipEventResponse {
                    body: Some(friendship_event_response::Body::Accept(AcceptResponse {
                        user: Some(User {
                            address: "0xAlice".to_owned(),
                        })
                    })),
                }
            );
            assert_eq!(user_to, "0xBob");
        }
        Err(_) => {
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
                FriendshipEventResponse {
                    body: Some(friendship_event_response::Body::Reject(RejectResponse {
                        user: Some(User {
                            address: "0xAlice".to_owned(),
                        })
                    })),
                }
            );
            assert_eq!(user_to, "0xBob");
        }
        Err(_) => {
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
                FriendshipEventResponse {
                    body: Some(friendship_event_response::Body::Cancel(CancelResponse {
                        user: Some(User {
                            address: "0xAlice".to_owned(),
                        })
                    })),
                }
            );
            assert_eq!(user_to, "0xBob");
        }
        Err(_) => {
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
                FriendshipEventResponse {
                    body: Some(friendship_event_response::Body::Delete(DeleteResponse {
                        user: Some(User {
                            address: "0xAlice".to_owned(),
                        })
                    })),
                }
            );
            assert_eq!(user_to, "0xBob");
        }
        Err(_) => {
            unreachable!("Parsing to delete response should not fail")
        }
    }
}
