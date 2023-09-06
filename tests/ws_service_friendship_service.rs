#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use social_service::{
        domain::{
            event::EventResponse, friendship_event::FriendshipEvent,
            friendship_event_validator::validate_new_event, friendship_status::FriendshipStatus,
            friendship_status_calculator::get_new_friendship_status,
        },
        entities::friendship_history::{
            FriendshipHistory, FriendshipMetadata, FriendshipRequestEvent,
        },
        friendships::{
            friendship_event_payload::Body, friendship_event_response, CancelPayload,
            FriendshipEventPayload, Payload, RequestPayload, UpdateFriendshipPayload, User,
        },
        ws::service::mapper::event::{
            event_response_as_update_response, friendship_requests_as_request_events_response,
            update_request_as_event_payload,
        },
    };
    use uuid::Uuid;

    #[test]
    fn test_friendship_requests_as_request_events() {
        // Database mock response
        let requests: Vec<FriendshipRequestEvent> = generate_request_events();

        // Authenticated user
        let user_id: String = "Pizarnik".to_owned();

        let result = friendship_requests_as_request_events_response(requests, user_id)
            .response
            .unwrap();

        match result {
            social_service::friendships::request_events_response::Response::Events(result) => {
                match result.outgoing {
                    Some(outgoing) => {
                        assert_eq!(outgoing.total, 1);

                        let first_request = outgoing.items.get(0);
                        match first_request {
                            Some(req) => {
                                assert_eq!(req.user.as_ref().unwrap().address, "PedroL");
                                assert!(req.created_at > 0);
                                assert!(req.message.is_none());
                            }
                            None => unreachable!("An error response was found"),
                        }
                    }
                    None => unreachable!("An error response was found"),
                }

                match result.incoming {
                    Some(incoming) => {
                        assert_eq!(incoming.total, 1);

                        let first_request = incoming.items.get(0);
                        match first_request {
                            Some(req) => {
                                assert_eq!(req.user.as_ref().unwrap().address, "Martha");
                                assert!(req.created_at > 0);
                                assert_eq!(req.message.as_ref().unwrap(), "Hey, let's be friends!");
                            }
                            None => unreachable!("An error response was found"),
                        }
                    }
                    None => unreachable!("An error response was found"),
                }
            }
            // Error responses
            _ => {
                unreachable!("An error response was found");
            }
        }
    }

    #[test]
    fn test_update_request_as_event_payload() {
        // Case 1: Request event
        let request = generate_update_friendship_payload(
            Body::Request(RequestPayload {
                message: Some("Let's be friends!".to_owned()),
                user: Some(User {
                    address: "Pizarnik".to_owned(),
                }),
            }),
            "Pizarnik".to_owned(),
        );

        let result = update_request_as_event_payload(request).unwrap();
        assert_eq!(result.friendship_event, FriendshipEvent::REQUEST);
        assert_eq!(
            result.request_event_message_body.unwrap(),
            "Let's be friends!"
        );
        assert_eq!(result.second_user, "Pizarnik");

        // Case 2: Cancel event
        let cancel = generate_update_friendship_payload(
            Body::Cancel(CancelPayload {
                user: Some(User {
                    address: "Pizarnik".to_owned(),
                }),
            }),
            "Pizarnik".to_owned(),
        );

        let result = update_request_as_event_payload(cancel).unwrap();
        assert_eq!(result.friendship_event, FriendshipEvent::CANCEL);
        assert!(result.request_event_message_body.is_none());
        assert_eq!(result.second_user, "Pizarnik");

        // Case 3: Event is None
        let none_event = UpdateFriendshipPayload {
            event: None,
            auth_token: None,
        };
        let result = update_request_as_event_payload(none_event);
        assert!(result.is_err());
    }

    #[test]
    fn test_event_response_as_update_response_request() {
        // Create an UpdateFriendshipPayload with a Request body
        let update_payload = generate_update_friendship_payload(
            Body::Request(RequestPayload {
                message: Some("Let's be friends!".to_owned()),
                user: Some(User {
                    address: "Pizarnik".to_owned(),
                }),
            }),
            "Pizarnik".to_owned(),
        );

        let event_response = EventResponse {
            user_id: "Pizarnik".to_owned(),
        };

        let result = event_response_as_update_response(update_payload, event_response, 1);
        assert!(result.is_ok());

        let update_response = result
            .expect("Failed to get result")
            .response
            .expect("Failed to get response");

        match update_response {
            social_service::friendships::update_friendship_response::Response::Event(
                update_response,
            ) => {
                assert!(update_response.body.is_some());

                let body = update_response.body.unwrap();
                match body {
                    friendship_event_response::Body::Request(request_response) => {
                        assert_eq!(
                            request_response.user.unwrap().address,
                            "Pizarnik".to_owned()
                        );
                        assert_eq!(
                            request_response.message.unwrap(),
                            "Let's be friends!".to_owned()
                        );
                    }
                    _ => panic!("Expected Request body"),
                }
            }
            // Error responses
            _ => {
                unreachable!("An error response was found");
            }
        }
    }

    #[test]
    fn test_validate_new_event() {
        // Case 1: No previous history
        let last_recorded_history = None;
        let new_event = FriendshipEvent::REQUEST;
        assert!(validate_new_event("Sussana", &last_recorded_history, new_event).is_ok());

        // Case 2: Previous history exists, new event is valid
        let last_recorded_history = Some(generate_friendship_history(
            FriendshipEvent::REQUEST,
            "Sussana",
            "2022-04-12 09:30:00",
        ));
        let new_event = FriendshipEvent::ACCEPT;
        assert!(validate_new_event("Juana", &last_recorded_history, new_event).is_ok());

        // Case 3: Previous history exists, new event is not valid
        let last_recorded_history = Some(generate_friendship_history(
            FriendshipEvent::ACCEPT,
            "Sussana",
            "2022-04-12 09:30:00",
        ));
        let new_event = FriendshipEvent::REQUEST;
        assert!(validate_new_event("Juana", &last_recorded_history, new_event).is_err());

        // Case 4: Previous history exists, new event is not different from the last recorded (aka invalid)
        let last_recorded_history = Some(generate_friendship_history(
            FriendshipEvent::REQUEST,
            "Sussana",
            "2022-04-12 09:30:00",
        ));
        let new_event = FriendshipEvent::REQUEST;
        assert!(validate_new_event("Sussana", &last_recorded_history, new_event).is_err());
    }

    #[test]
    fn test_validate_and_get_new_friendship_status() {
        let acting_user = "Pizarnik";

        // Case 1: Requesting friendship when no history exists
        let event = FriendshipEvent::REQUEST;
        validate_new_event(acting_user, &None, event).unwrap();
        let result = get_new_friendship_status(acting_user, event);
        assert_eq!(result, FriendshipStatus::Requested(acting_user.to_string()));

        // Case 2: Accepting friendship after a request was sent
        let event = FriendshipEvent::ACCEPT;
        let last_recorded_history = Some(generate_friendship_history(
            FriendshipEvent::REQUEST,
            "OtherUser",
            "2022-04-12 09:30:00",
        ));
        validate_new_event(acting_user, &last_recorded_history, event).unwrap();
        let result = get_new_friendship_status(acting_user, event);
        assert_eq!(result, FriendshipStatus::Friends);

        // Case 3: Deleting friendship
        let event = FriendshipEvent::DELETE;
        let last_recorded_history = Some(generate_friendship_history(
            FriendshipEvent::ACCEPT,
            "OtherUser",
            "2022-04-12 09:30:00",
        ));
        validate_new_event(acting_user, &last_recorded_history, event).unwrap();
        let result = get_new_friendship_status(acting_user, event);
        assert_eq!(result, FriendshipStatus::NotFriends);
    }

    fn generate_request_events() -> Vec<FriendshipRequestEvent> {
        let timestamp_str = "2022-04-12 09:30:00";
        let timestamp = NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S").unwrap();

        vec![
            FriendshipRequestEvent {
                acting_user: "Martha".to_owned(),
                address_1: "Martha".to_owned(),
                address_2: "Pizarnik".to_owned(),
                timestamp,
                metadata: Some(sqlx::types::Json(FriendshipMetadata {
                    message: Some("Hey, let's be friends!".to_owned()),
                    synapse_room_id: None,
                    migrated_from_synapse: None,
                })),
            },
            FriendshipRequestEvent {
                acting_user: "Pizarnik".to_owned(),
                address_1: "PedroL".to_owned(),
                address_2: "Pizarnik".to_owned(),
                timestamp,
                metadata: None,
            },
        ]
    }

    fn generate_friendship_history(
        event: FriendshipEvent,
        acting_user: &str,
        timestamp_str: &str,
    ) -> FriendshipHistory {
        let timestamp = NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S").unwrap();
        FriendshipHistory {
            friendship_id: Uuid::new_v4(),
            event,
            acting_user: acting_user.to_string(),
            timestamp,
            metadata: None,
        }
    }

    fn generate_update_friendship_payload(event: Body, user: String) -> UpdateFriendshipPayload {
        UpdateFriendshipPayload {
            event: Some(FriendshipEventPayload { body: Some(event) }),
            auth_token: Some(Payload {
                synapse_token: Some(format!("{user}Token")),
            }),
        }
    }
}
