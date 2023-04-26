#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use social_service::{
        entities::friendship_history::{
            FriendshipHistory, FriendshipMetadata, FriendshipRequestEvent,
        },
        friendships::friendship_event_payload::Body,
        friendships::friendship_event_response,
        friendships::CancelPayload,
        friendships::FriendshipEventPayload,
        friendships::Payload,
        friendships::RequestEvents,
        friendships::RequestPayload,
        friendships::UpdateFriendshipPayload,
        friendships::User,
        models::friendship_event::FriendshipEvent,
        models::friendship_status::FriendshipStatus,
        ws::service::{
            friendship_event_validator::validate_new_event,
            friendship_status_calculator::get_new_friendship_status,
            mapper::{
                event_response_as_update_response, friendship_requests_as_request_events,
                update_request_as_event_payload,
            },
            types::EventResponse,
        },
    };
    use uuid::Uuid;

    #[test]
    fn test_friendship_requests_as_request_events() {
        // Database mock response
        let requests: Vec<FriendshipRequestEvent> = generate_request_events();

        // Authenticated user
        let user_id: String = "Pizarnik".to_string();

        let mut result: RequestEvents = friendship_requests_as_request_events(requests, user_id);

        assert_eq!(result.outgoing.unwrap().total, 1);
        assert_eq!(result.incoming.clone().unwrap().total, 1);

        let incoming_requests = result.incoming.take();
        if let Some(incoming_requests) = incoming_requests {
            let incoming_request = incoming_requests.items.get(0).unwrap();
            assert_eq!(incoming_request.user.as_ref().unwrap().address, "Martha");
            assert!(incoming_request.created_at > 0);
            assert_eq!(
                incoming_request.message.clone().unwrap_or_default(),
                "Hey, let's be friends!"
            );
        }
    }

    #[test]
    fn test_update_request_as_event_payload() {
        // Case 1: Request event
        let request = generate_update_friendship_payload(
            Body::Request(RequestPayload {
                message: Some("Let's be friends!".to_string()),
                user: Some(User {
                    address: "Pizarnik".to_string(),
                }),
            }),
            "Pizarnik".to_string(),
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
                    address: "Pizarnik".to_string(),
                }),
            }),
            "Pizarnik".to_string(),
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
                message: Some("Let's be friends!".to_string()),
                user: Some(User {
                    address: "Pizarnik".to_string(),
                }),
            }),
            "Pizarnik".to_string(),
        );

        let event_response = EventResponse {
            user_id: "Pizarnik".to_string(),
        };

        let result = event_response_as_update_response(update_payload, event_response);
        assert!(result.is_ok());

        let update_response = result.unwrap();
        assert!(update_response.event.is_some());

        let event = update_response.event.unwrap();
        assert!(event.body.is_some());

        let body = event.body.unwrap();
        match body {
            friendship_event_response::Body::Request(request_response) => {
                assert_eq!(
                    request_response.user.unwrap().address,
                    "Pizarnik".to_string()
                );
                assert_eq!(
                    request_response.message.unwrap(),
                    "Let's be friends!".to_string()
                );
            }
            _ => panic!("Expected Request body"),
        }
    }

    #[test]
    fn test_validate_new_event() {
        // Case 1: No previous history
        let last_recorded_history = None;
        let new_event = FriendshipEvent::REQUEST;
        assert!(validate_new_event(&last_recorded_history, new_event).is_ok());

        // Case 2: Previous history exists, new event is valid
        let last_recorded_history = Some(generate_friendship_history(
            FriendshipEvent::REQUEST,
            "Sussana",
            "2022-04-12 09:30:00",
        ));
        let new_event = FriendshipEvent::ACCEPT;
        assert!(validate_new_event(&last_recorded_history, new_event).is_ok());

        // Case 3: Previous history exists, new event is not valid
        let last_recorded_history = Some(generate_friendship_history(
            FriendshipEvent::ACCEPT,
            "Sussana",
            "2022-04-12 09:30:00",
        ));
        let new_event = FriendshipEvent::REQUEST;
        assert!(validate_new_event(&last_recorded_history, new_event).is_err());

        // Case 4: Previous history exists, new event is not different from the last recorded (aka invalid)
        let last_recorded_history = Some(generate_friendship_history(
            FriendshipEvent::REQUEST,
            "Sussana",
            "2022-04-12 09:30:00",
        ));
        let new_event = FriendshipEvent::REQUEST;
        assert!(validate_new_event(&last_recorded_history, new_event).is_err());
    }

    #[test]
    fn test_get_new_friendship_status() {
        let acting_user = "Pizarnik";
        let last_recorded_history = None;

        // Case 1: Requesting friendship when no history exists
        let result = get_new_friendship_status(
            acting_user,
            &last_recorded_history,
            FriendshipEvent::REQUEST,
        )
        .unwrap();
        assert_eq!(result, FriendshipStatus::Requested(acting_user.to_string()));

        // Case 2: Accepting friendship after a request was sent
        let last_recorded_history = Some(generate_friendship_history(
            FriendshipEvent::REQUEST,
            "OtherUser",
            "2022-04-12 09:30:00",
        ));
        let result =
            get_new_friendship_status(acting_user, &last_recorded_history, FriendshipEvent::ACCEPT)
                .unwrap();
        assert_eq!(result, FriendshipStatus::Friends);

        // Case 3: Deleting friendship
        let last_recorded_history = Some(generate_friendship_history(
            FriendshipEvent::ACCEPT,
            "OtherUser",
            "2022-04-12 09:30:00",
        ));
        let result =
            get_new_friendship_status(acting_user, &last_recorded_history, FriendshipEvent::DELETE)
                .unwrap();
        assert_eq!(result, FriendshipStatus::NotFriends);
    }

    fn generate_request_events() -> Vec<FriendshipRequestEvent> {
        let timestamp_str = "2022-04-12 09:30:00";
        let timestamp = NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S").unwrap();

        vec![
            FriendshipRequestEvent {
                acting_user: "Martha".to_string(),
                address_1: "Martha".to_string(),
                address_2: "Pizarnik".to_string(),
                timestamp,
                metadata: Some(sqlx::types::Json(FriendshipMetadata {
                    message: Some("Hey, let's be friends!".to_string()),
                    synapse_room_id: None,
                    migrated_from_synapse: None,
                })),
            },
            FriendshipRequestEvent {
                acting_user: "Pizarnik".to_string(),
                address_1: "PedroL".to_string(),
                address_2: "Pizarnik".to_string(),
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
