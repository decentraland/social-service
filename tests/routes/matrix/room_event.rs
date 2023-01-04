#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use crate::common::*;

    use social_service::{
        components::synapse::{RoomMember, RoomMembersResponse},
        entities::friendships::FriendshipRepositoryImplementation,
        routes::synapse::room_events::{FriendshipEvent, RoomEventRequestBody, RoomEventResponse},
    };
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    const ROOM_STATE_URI: &str =
        "/_matrix/client/r0/rooms/a_room_id/state/org.decentraland.friendship";
    const ROOM_MEMBERS_URI: &str = "/_matrix/client/r0/rooms/a_room_id/members";

    async fn get_synapse_mocked_server_with_room(
        token_to_user_id: HashMap<String, String>,
        room_members: (String, String),
    ) -> MockServer {
        let synapse_server = who_am_i_synapse_mock_server(token_to_user_id).await;

        let room_members_response = RoomMembersResponse {
            chunk: vec![
                RoomMember {
                    room_id: "a_room_id".to_string(),
                    r#type: "".to_string(),
                    user_id: room_members.0,
                },
                RoomMember {
                    room_id: "a_room_id".to_string(),
                    r#type: "".to_string(),
                    user_id: room_members.1,
                },
            ],
        };

        let room_state_response = RoomEventResponse {
            event_id: "anState".to_string(),
        };

        Mock::given(method("GET"))
            .and(path(ROOM_MEMBERS_URI))
            .respond_with(ResponseTemplate::new(200).set_body_json(room_members_response))
            .mount(&synapse_server)
            .await;
        Mock::given(method("PUT"))
            .and(path(ROOM_STATE_URI))
            .respond_with(ResponseTemplate::new(200).set_body_json(room_state_response))
            .mount(&synapse_server)
            .await;

        synapse_server
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_cancel() {
        let user_1_id = "LALA";
        let user_2_id = "LELE";

        // TODO!: Implement a function that returns tokens to prevent collision between tests
        let token_1 = "LALA-1";
        let token_2 = "LELE-2";

        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(token_1.to_string(), user_1_id.to_string());
        token_to_user_id.insert(token_2.to_string(), user_2_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (user_1_id.to_string(), user_2_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        let header = ("authorization", format!("Bearer {}", token_1));

        // test 1

        // user A request user B
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::REQUEST,
        };

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let repos = db.db_repos.unwrap();

        let result = repos
            .friendships
            .get_friendship((user_1_id, user_2_id), None)
            .await
            .0
            .unwrap()
            .unwrap();

        // assert not friends in db yet
        assert!(!result.is_active);

        // user A cancel request for user B
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::CANCEL,
        };

        let header = ("authorization", format!("Bearer {}", token_1));

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let result = repos
            .friendships
            .get_friendship((user_1_id, user_2_id), None)
            .await
            .0
            .unwrap()
            .unwrap();

        // assert not friends in db yet
        assert!(!result.is_active);
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_reject() {
        let user_1_id = "LALA";
        let user_2_id = "LELE";

        // TODO!: Implement a function that returns tokens to prevent collision between tests
        let token_1 = "LALA-1";
        let token_2 = "LELE-2";

        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(token_1.to_string(), user_1_id.to_string());
        token_to_user_id.insert(token_2.to_string(), user_2_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (user_1_id.to_string(), user_2_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        let header = ("authorization", format!("Bearer {}", token_1));

        // test 2

        // user A request user B
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::REQUEST,
        };

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let _ = actix_web::test::call_service(&app, req).await;

        let repos = db.db_repos.unwrap();

        let result = repos
            .friendships
            .get_friendship((user_1_id, user_2_id), None)
            .await
            .0
            .unwrap()
            .unwrap();

        let result = repos
            .friendship_history
            .get_last_history_for_friendship(result.id, None)
            .await
            .0
            .unwrap()
            .unwrap();

        // assert last history is request
        assert_eq!(result.event, FriendshipEvent::REQUEST);

        // user B reject user A
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::REJECT,
        };

        let header = ("authorization", format!("Bearer {}", token_2));

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let _ = actix_web::test::call_service(&app, req).await;

        let result = repos
            .friendships
            .get_friendship((user_1_id, user_2_id), None)
            .await
            .0
            .unwrap()
            .unwrap();

        // assert not friends in db yet
        assert!(!result.is_active);

        let result = repos
            .friendship_history
            .get_last_history_for_friendship(result.id, None)
            .await
            .0
            .unwrap()
            .unwrap();

        // assert last history is reject
        assert_eq!(result.event, FriendshipEvent::REJECT);
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_accept() {
        let user_1_id = "LALA";
        let user_2_id = "LELE";

        // TODO!: Implement a function that returns tokens to prevent collision between tests
        let token_1 = "LALA-1";
        let token_2 = "LELE-2";

        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(token_1.to_string(), user_1_id.to_string());
        token_to_user_id.insert(token_2.to_string(), user_2_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (user_1_id.to_string(), user_2_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // test 3

        // user A request user B
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::REQUEST,
        };

        let header = ("authorization", format!("Bearer {}", token_1));

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let _ = actix_web::test::call_service(&app, req).await;

        // user B accept user A
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::ACCEPT,
        };

        let header = ("authorization", format!("Bearer {}", token_2));

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let _ = actix_web::test::call_service(&app, req).await;

        let repos = db.db_repos.unwrap();

        let result = repos
            .friendships
            .get_friendship((user_1_id, user_2_id), None)
            .await
            .0
            .unwrap()
            .unwrap();

        // assert friends in db
        assert!(result.is_active);

        let result = repos
            .friendship_history
            .get_last_history_for_friendship(result.id, None)
            .await
            .0
            .unwrap()
            .unwrap();

        // assert last history is accept
        assert_eq!(result.event, FriendshipEvent::ACCEPT);
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_accept_delete() {
        let user_1_id = "LALA";
        let user_2_id = "LELE";

        // TODO!: Implement a function that returns tokens to prevent collision between tests
        let token_1 = "LALA-1";
        let token_2 = "LELE-2";

        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(token_1.to_string(), user_1_id.to_string());
        token_to_user_id.insert(token_2.to_string(), user_2_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (user_1_id.to_string(), user_2_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // test 4

        // user A request user B
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::REQUEST,
        };

        let header = ("authorization", format!("Bearer {}", token_1));

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let _ = actix_web::test::call_service(&app, req).await;

        // user B accept user A
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::ACCEPT,
        };

        let header = ("authorization", format!("Bearer {}", token_2));

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let _ = actix_web::test::call_service(&app, req).await;

        let repos = db.db_repos.unwrap();

        let result = repos
            .friendships
            .get_friendship((user_1_id, user_2_id), None)
            .await
            .0
            .unwrap()
            .unwrap();

        // assert friends in db
        assert!(result.is_active);

        // user B delete user A
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::DELETE,
        };

        let header = ("authorization", format!("Bearer {}", token_2));

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let _ = actix_web::test::call_service(&app, req).await;

        // assert not friends in db
        let result = repos
            .friendships
            .get_friendship((user_1_id, user_2_id), None)
            .await
            .0
            .unwrap()
            .unwrap();

        // assert not friends in db anymore
        assert!(!result.is_active);

        let result = repos
            .friendship_history
            .get_last_history_for_friendship(result.id, None)
            .await
            .0
            .unwrap()
            .unwrap();

        // assert last history is delete by B
        assert_eq!(result.acting_user, user_2_id);
        assert_eq!(result.event, FriendshipEvent::DELETE);
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_request_should_400() {
        let user_1_id = "LALA";
        let user_2_id = "LELE";

        // TODO!: Implement a function that returns tokens to prevent collision between tests
        let token_1 = "LALA-1";
        let token_2 = "LELE-2";

        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(token_1.to_string(), user_1_id.to_string());
        token_to_user_id.insert(token_2.to_string(), user_2_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (user_1_id.to_string(), user_2_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::REQUEST,
        };

        let header = ("authorization", format!("Bearer {}", token_1));

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let _ = actix_web::test::call_service(&app, req).await;
        // user B request user A
        let body = RoomEventRequestBody {
            r#type: FriendshipEvent::REQUEST,
        };

        let header = ("authorization", format!("Bearer {}", token_1));

        let req = actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request();

        let response = actix_web::test::call_service(&app, req).await;

        // endpoint returns error bad request

        assert_eq!(response.status(), 400);
    }
}
