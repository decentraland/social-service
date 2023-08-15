#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use crate::common::*;

    use actix_http::Request;

    use social_service::{
        api::routes::synapse::room_events::{RoomEventRequestBody, RoomEventResponse},
        components::{
            database::DBRepositories,
            synapse::{RoomMember, RoomMembersResponse},
        },
        domain::friendship_event::FriendshipEvent,
        entities::friendships::{Friendship, FriendshipRepositoryImplementation},
    };
    use uuid::Uuid;
    use wiremock::{
        matchers::{method, path, path_regex},
        Mock, MockServer, ResponseTemplate,
    };

    const ROOM_STATE_URI: &str =
        "/_matrix/client/r0/rooms/a_room_id/state/org.decentraland.friendship";
    const ROOM_MEMBERS_URI: &str = "/_matrix/client/r0/rooms/a_room_id/members";
    const ROOM_MESSAGE_EVENT_URI: &str =
        r"^/_matrix/client/r0/rooms/a_room_id/send/m.room.message/[m\.0-9-~_]{1,}"; // Matches m.1675968342200

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
                    state_key: room_members.0,
                    social_user_id: None,
                    user_id: "".to_string(),
                },
                RoomMember {
                    room_id: "a_room_id".to_string(),
                    r#type: "".to_string(),
                    state_key: room_members.1,
                    social_user_id: None,
                    user_id: "".to_string(),
                },
            ],
        };

        let room_state_response = RoomEventResponse {
            event_id: "anState".to_string(),
        };

        let room_message_event_response = RoomEventResponse {
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
        Mock::given(method("PUT"))
            .and(path_regex(ROOM_MESSAGE_EVENT_URI))
            .respond_with(ResponseTemplate::new(200).set_body_json(room_message_event_response))
            .mount(&synapse_server)
            .await;

        synapse_server
    }

    struct TestUser<'a> {
        user_id: &'a str,
        social_user_id: &'a str,
        token: &'a str,
    }

    // TODO!: Implement a function that returns tokens to prevent collision between tests
    const USER_A: TestUser = TestUser {
        user_id: "@LALA",
        social_user_id: "LALA",
        token: "LALA-1",
    };
    const USER_B: TestUser = TestUser {
        user_id: "@LELE",
        social_user_id: "LELE",
        token: "LELE-2",
    };

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_cancel() {
        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(USER_A.token.to_string(), USER_A.user_id.to_string());
        token_to_user_id.insert(USER_B.token.to_string(), USER_B.user_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A.user_id.to_string(), USER_B.user_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let req = get_request(USER_A.token, FriendshipEvent::REQUEST, None);
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let repos = db.db_repos.unwrap();

        // assert not friends in db yet
        assert_and_get_friendship_from_db(
            &repos,
            (USER_A.social_user_id, USER_B.social_user_id),
            false,
        )
        .await;

        // user A cancel request for user B
        let req = get_request(USER_A.token, FriendshipEvent::CANCEL, None);

        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // assert not friends in db yet
        assert_and_get_friendship_from_db(
            &repos,
            (USER_A.social_user_id, USER_B.social_user_id),
            false,
        )
        .await;
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_reject() {
        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(USER_A.token.to_string(), USER_A.user_id.to_string());
        token_to_user_id.insert(USER_B.token.to_string(), USER_B.user_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A.user_id.to_string(), USER_B.user_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let message_body = "hey, wanna be friends with me?".to_string();
        let req = get_request(
            USER_A.token,
            FriendshipEvent::REQUEST,
            Some(message_body.clone()),
        );
        let _ = actix_web::test::call_service(&app, req).await;

        let repos = db.db_repos.unwrap();

        let result = assert_and_get_friendship_from_db(
            &repos,
            (USER_A.social_user_id, USER_B.social_user_id),
            false,
        )
        .await;

        // assert last history is request
        assert_last_history_from_db(
            &repos,
            result.id,
            USER_A.social_user_id,
            FriendshipEvent::REQUEST,
            Some(message_body),
        )
        .await;

        // user B reject user A
        let req = get_request(USER_B.token, FriendshipEvent::REJECT, None);
        let _ = actix_web::test::call_service(&app, req).await;

        // assert not friends in db yet
        let result = assert_and_get_friendship_from_db(
            &repos,
            (USER_A.social_user_id, USER_B.social_user_id),
            false,
        )
        .await;

        // assert last history is reject
        assert_last_history_from_db(
            &repos,
            result.id,
            USER_B.social_user_id,
            FriendshipEvent::REJECT,
            None,
        )
        .await;
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_accept() {
        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(USER_A.token.to_string(), USER_A.user_id.to_string());
        token_to_user_id.insert(USER_B.token.to_string(), USER_B.user_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A.user_id.to_string(), USER_B.user_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let req = get_request(USER_A.token, FriendshipEvent::REQUEST, None);
        let _ = actix_web::test::call_service(&app, req).await;

        let repos = db.db_repos.unwrap();

        // user B accept user A
        let req = get_request(USER_B.token, FriendshipEvent::ACCEPT, None);
        let _ = actix_web::test::call_service(&app, req).await;

        // assert friends in db
        let result = assert_and_get_friendship_from_db(
            &repos,
            (USER_A.social_user_id, USER_B.social_user_id),
            true,
        )
        .await;

        // assert last history is accept
        assert_last_history_from_db(
            &repos,
            result.id,
            USER_B.social_user_id,
            FriendshipEvent::ACCEPT,
            None,
        )
        .await;
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_accept_delete() {
        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(USER_A.token.to_string(), USER_A.user_id.to_string());
        token_to_user_id.insert(USER_B.token.to_string(), USER_B.user_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A.user_id.to_string(), USER_B.user_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let req = get_request(USER_A.token, FriendshipEvent::REQUEST, None);
        let _ = actix_web::test::call_service(&app, req).await;

        // user B accept user A
        let req = get_request(USER_B.token, FriendshipEvent::ACCEPT, None);
        let _ = actix_web::test::call_service(&app, req).await;

        let repos = db.db_repos.unwrap();

        // assert friends in db
        assert_and_get_friendship_from_db(
            &repos,
            (USER_A.social_user_id, USER_B.social_user_id),
            true,
        )
        .await;

        // user B delete user A
        let req = get_request(USER_B.token, FriendshipEvent::DELETE, None);
        let _ = actix_web::test::call_service(&app, req).await;

        // assert not friends in db anymore
        let result = assert_and_get_friendship_from_db(
            &repos,
            (USER_A.social_user_id, USER_B.social_user_id),
            false,
        )
        .await;

        // assert last history is delete by B
        assert_last_history_from_db(
            &repos,
            result.id,
            USER_B.social_user_id,
            FriendshipEvent::DELETE,
            None,
        )
        .await;
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_request_should_400() {
        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(USER_A.token.to_string(), USER_A.user_id.to_string());
        token_to_user_id.insert(USER_B.token.to_string(), USER_B.user_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A.user_id.to_string(), USER_B.user_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let req = get_request(USER_A.token, FriendshipEvent::REQUEST, None);
        let _ = actix_web::test::call_service(&app, req).await;

        // user B request user A
        let req = get_request(USER_B.token, FriendshipEvent::REQUEST, None);
        let response = actix_web::test::call_service(&app, req).await;

        // endpoint returns error bad request
        assert_eq!(response.status(), 400);
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request() {
        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(USER_A.token.to_string(), USER_A.user_id.to_string());
        token_to_user_id.insert(USER_B.token.to_string(), USER_B.user_id.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A.user_id.to_string(), USER_B.user_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        let message_body = "hey, wanna be friends with me?".to_string();
        // user A request user B
        let req = get_request(
            USER_A.token,
            FriendshipEvent::REQUEST,
            Some(message_body.clone()),
        );
        let _ = actix_web::test::call_service(&app, req).await;

        let repos = db.db_repos.unwrap();

        let result = assert_and_get_friendship_from_db(
            &repos,
            (USER_A.social_user_id, USER_B.social_user_id),
            false,
        )
        .await;

        // assert last history is request
        assert_last_history_from_db(
            &repos,
            result.id,
            USER_A.social_user_id,
            FriendshipEvent::REQUEST,
            Some(message_body),
        )
        .await;
    }

    fn get_request(token: &str, event_type: FriendshipEvent, body: Option<String>) -> Request {
        let body = RoomEventRequestBody {
            r#type: event_type,
            message: body,
        };

        let header = ("authorization", format!("Bearer {token}"));

        actix_web::test::TestRequest::put()
            .uri(ROOM_STATE_URI)
            .insert_header(header)
            .set_json(body)
            .to_request()
    }

    async fn assert_and_get_friendship_from_db(
        repos: &DBRepositories,
        addresses: (&str, &str),
        is_active: bool,
    ) -> Friendship {
        let result = repos
            .friendships
            .get_friendship(addresses, None)
            .await
            .0
            .unwrap()
            .unwrap();

        assert_eq!(result.is_active, is_active);

        result
    }

    async fn assert_last_history_from_db(
        repos: &DBRepositories,
        friendship_id: Uuid,
        expected_acting_user: &str,
        event_type: FriendshipEvent,
        message: Option<String>,
    ) {
        let result = repos
            .friendship_history
            .get_last_history_for_friendship(friendship_id, None)
            .await
            .0
            .unwrap()
            .unwrap();

        assert_eq!(result.acting_user, expected_acting_user);
        assert_eq!(result.event, event_type);
        let message_body = result
            .metadata
            .and_then(|j| j.message.clone())
            .unwrap_or("".to_string());
        assert_eq!(message_body, message.unwrap_or("".to_string()));
    }
}
