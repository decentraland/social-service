#[cfg(test)]
mod tests {
    use crate::common::*;
    use actix_web::{test};
    use social_service::{
        components::{
            synapse::{RoomMember, RoomMembersResponse},
        },
        entities::friendships::FriendshipRepositoryImplementation,
        routes::{synapse::room_events::{FriendshipEvent, RoomEventRequestBody, RoomEventResponse}, v1::error::ErrorResponse},
    };
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    const ROOM_STATE_URI: &str =
        "/_matrix/client/r0/rooms/a_room_id/state/org.decentraland.friendship";
    const ROOM_MEMBERS_URI: &str = "/_matrix/client/r0/rooms/a_room_id/members";

    async fn get_synapse_mocked_server_with_room(
        user_id: String,
        room_members: (String, String),
    ) -> MockServer {
        let synapse_server = who_am_i_synapse_mock_server(user_id.to_string()).await;

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
    async fn test_friendship_lifecycle() {

        let user_1_id = "0xa";
        let user_2_id = "0xb";
        let synapse_server = get_synapse_mocked_server_with_room(
            user_1_id.to_string(),
            (user_1_id.to_string(), user_2_id.to_string()),
        )
        .await;

        let mut config = get_configuration().await;
        config.synapse.url = synapse_server.uri();
        let db = create_db_component(&Some(config)).await;

        let app = actix_web::test::init_service(get_app(config, None).await).await;

        let token = "a1b2c3d4";
        let header = ("authorization", format!("Bearer {}", token));

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

        // let friendships_response: ErrorResponse = test::read_body_json(resp).await;
        // println!("ACA fallo {}", friendships_response.message);
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
        // assert not friends in db yet

        // test 2

        // user A request user B
        // assert not friends in db yet
        // assert history only has request

        // user B reject user A
        // assert not friends in db yet
        // assert history has request and reject

        // test 3

        // user A request user B
        // assert not friends in db yet
        // assert history only has request

        // user B accept user A
        // assert friends in db
        // assert history has request and accept

        // test 4

        // user A request user B
        // user B accept user A
        // assert friends in db
        // user B delete user A
        // assert not friends in db
        // assert history has request, accept, delete by B

        // test 5

        // user A request user B
        // user B request user A
        // assert friends in db
        // assert history has request and accept
    }
}
