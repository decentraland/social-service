#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use social_service::{
        entities::friendship_history::{FriendshipMetadata, FriendshipRequestEvent},
        ws::service::friendships_service::map_request_events,
        RequestEvents,
    };

    #[test]
    fn test_map_request_events() {
        // Database mock response
        let requests: Vec<FriendshipRequestEvent> = generate_request_events();

        // Authenticated user
        let user_id: String = "Pizarnik".to_string();

        // Function to test
        let result: RequestEvents = map_request_events(requests, user_id);

        assert_eq!(result.outgoing.unwrap().total, 1);
        assert_eq!(result.incoming.clone().unwrap().total, 1);

        let incoming_requests = result.incoming.clone().take();
        if let Some(incoming_requests) = incoming_requests {
            let incoming_request = incoming_requests.items.get(0).unwrap();
            assert_eq!(incoming_request.user.as_ref().unwrap().address, "Martha");
            assert_eq!(incoming_request.created_at > 0, true);
            assert_eq!(
                incoming_request.message.clone().unwrap_or_default(),
                "Hey, let's be friends!"
            );
        }
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
}
