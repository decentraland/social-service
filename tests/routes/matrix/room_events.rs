#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use crate::common::*;

    use actix_http::Request;

    use social_service::{
        components::{
            database::DBRepositories,
            synapse::{RoomMember, RoomMembersResponse},
        },
        entities::friendships::{Friendship, FriendshipRepositoryImplementation},
        routes::synapse::room_events::{FriendshipEvent, RoomEventRequestBody, RoomEventResponse},
    };
    use uuid::Uuid;
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


    const USER_A_ID: &str = "LALA";
    const USER_B_ID: &str = "LELE";

    // TODO!: Implement a function that returns tokens to prevent collision between tests
    const TOKEN_A: &str = "LALA-1";
    const TOKEN_B: &str = "LELE-2";

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_cancel() {

        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(TOKEN_A.to_string(), USER_A_ID.to_string());
        token_to_user_id.insert(TOKEN_B.to_string(), USER_B_ID.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A_ID.to_string(), USER_B_ID.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let req = get_request(TOKEN_A, FriendshipEvent::REQUEST);
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let repos = db.db_repos.unwrap();

        // assert not friends in db yet
        assert_and_get_friendship_from_db(&repos, (USER_A_ID, USER_B_ID), false).await;

        // user A cancel request for user B
        let req = get_request(TOKEN_A, FriendshipEvent::CANCEL);

        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // assert not friends in db yet
        assert_and_get_friendship_from_db(&repos, (USER_A_ID, USER_B_ID), false).await;
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_reject() {

        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(TOKEN_A.to_string(), USER_A_ID.to_string());
        token_to_user_id.insert(TOKEN_B.to_string(), USER_B_ID.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A_ID.to_string(), USER_B_ID.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let req = get_request(TOKEN_A, FriendshipEvent::REQUEST);
        let _ = actix_web::test::call_service(&app, req).await;

        let repos = db.db_repos.unwrap();

        let result = assert_and_get_friendship_from_db(&repos, (USER_A_ID, USER_B_ID), false).await;

        // assert last history is request
        assert_last_history_from_db(&repos, result.id, USER_A_ID, FriendshipEvent::REQUEST).await;

        // user B reject user A
        let req = get_request(TOKEN_B, FriendshipEvent::REJECT);
        let _ = actix_web::test::call_service(&app, req).await;

        // assert not friends in db yet
        let result = assert_and_get_friendship_from_db(&repos, (USER_A_ID, USER_B_ID), false).await;

        // assert last history is reject
        assert_last_history_from_db(&repos, result.id, USER_B_ID, FriendshipEvent::REJECT).await;
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_accept() {

        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(TOKEN_A.to_string(), USER_A_ID.to_string());
        token_to_user_id.insert(TOKEN_B.to_string(), USER_B_ID.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A_ID.to_string(), USER_B_ID.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let req = get_request(TOKEN_A, FriendshipEvent::REQUEST);
        let _ = actix_web::test::call_service(&app, req).await;

        // user B accept user A
        let req = get_request(TOKEN_B, FriendshipEvent::ACCEPT);
        let _ = actix_web::test::call_service(&app, req).await;

        let repos = db.db_repos.unwrap();

        // assert friends in db
        let result = assert_and_get_friendship_from_db(&repos, (USER_A_ID, USER_B_ID), true).await;

        // assert last history is accept
        assert_last_history_from_db(&repos, result.id, USER_B_ID, FriendshipEvent::ACCEPT).await;
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_accept_delete() {

        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(TOKEN_A.to_string(), USER_A_ID.to_string());
        token_to_user_id.insert(TOKEN_B.to_string(), USER_B_ID.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A_ID.to_string(), USER_B_ID.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(Some(&config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let req = get_request(TOKEN_A, FriendshipEvent::REQUEST);
        let _ = actix_web::test::call_service(&app, req).await;

        // user B accept user A
        let req = get_request(TOKEN_B, FriendshipEvent::ACCEPT);
        let _ = actix_web::test::call_service(&app, req).await;

        let repos = db.db_repos.unwrap();

        // assert friends in db
        assert_and_get_friendship_from_db(&repos, (USER_A_ID, USER_B_ID), true).await;

        // user B delete user A
        let req = get_request(TOKEN_B, FriendshipEvent::DELETE);
        let _ = actix_web::test::call_service(&app, req).await;

        // assert not friends in db anymore
        let result = assert_and_get_friendship_from_db(&repos, (USER_A_ID, USER_B_ID), false).await;

        // assert last history is delete by B
        assert_last_history_from_db(&repos, result.id, USER_B_ID, FriendshipEvent::DELETE).await;
    }

    #[actix_web::test]
    async fn test_friendship_lifecycle_request_request_should_400() {

        let mut token_to_user_id: HashMap<String, String> = HashMap::new();
        token_to_user_id.insert(TOKEN_A.to_string(), USER_A_ID.to_string());
        token_to_user_id.insert(TOKEN_B.to_string(), USER_B_ID.to_string());

        let synapse_server = get_synapse_mocked_server_with_room(
            token_to_user_id,
            (USER_A_ID.to_string(), USER_B_ID.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        // user A request user B
        let req = get_request(TOKEN_A, FriendshipEvent::REQUEST);
        let _ = actix_web::test::call_service(&app, req).await;

        // user B request user A
        let req = get_request(TOKEN_B, FriendshipEvent::REQUEST);
        let response = actix_web::test::call_service(&app, req).await;

        // endpoint returns error bad request
        assert_eq!(response.status(), 400);
    }

    fn get_request(token: &str, event_type: FriendshipEvent) -> Request {
        let body = RoomEventRequestBody { r#type: event_type };

        let header = ("authorization", format!("Bearer {}", token));

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
    }
}
