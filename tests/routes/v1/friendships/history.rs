use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use actix_http::StatusCode;
use actix_web::test;
use actix_web::web::Data;

use social_service::entities::friendship_history::FriendshipMetadata;
use social_service::get_app_router;
use social_service::routes::v1::friendships::types::MessageRequestEventResponse;
use sqlx::types::Json;

use social_service::components::app::AppComponents;

use crate::common::*;
use crate::routes::v1::friendships::utils::{add_friendship, create_friendship_history};

#[actix_rt::test]
async fn test_get_sent_messages_request_event() {
    let user_id = "a_user_id";
    let other_user_id = "other_user_id";

    let room_message_body = Some("hi, wanna be friends?");
    let metadata = Json::from(sqlx::types::Json(FriendshipMetadata {
        message: room_message_body.map(|body| body.to_string()),
        synapse_room_id: Some("a room_id".to_string()),
        migrated_from_synapse: Some(true),
    }));

    let metadata_without_body = Json::from(sqlx::types::Json(FriendshipMetadata {
        message: None,
        synapse_room_id: Some("a room_id".to_string()),
        migrated_from_synapse: None,
    }));

    let token = "my-token";

    let mut token_to_user_id: HashMap<String, String> = HashMap::new();
    token_to_user_id.insert(token.to_string(), user_id.to_string());

    let mock_server = who_am_i_synapse_mock_server(token_to_user_id).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let router = get_app_router(&app_data);

    let app = test::init_service(router).await;

    // Add friendship entry
    let friendship_id = add_friendship(&app_data.db, (user_id, other_user_id), true).await;

    // Create friendship request entry with metadata that contains the key `message`
    create_friendship_history(
        &app_data.db,
        friendship_id,
        "\"request\"",
        user_id,
        Some(metadata),
    )
    .await;

    // Create friendship request entry with metadata that does not contain the key `message`
    create_friendship_history(
        &app_data.db,
        friendship_id,
        "\"request\"",
        user_id,
        Some(metadata_without_body),
    )
    .await;

    // Create friendship request entry without metadata
    create_friendship_history(&app_data.db, friendship_id, "\"request\"", user_id, None).await;

    let url = format!(
        "/v1/friendships/{friendship_id}/request-events/messages?timestamp_from={}&timestamp_to={}",
        1662921288,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let header = ("authorization", format!("Bearer {}", token));
    let req = test::TestRequest::get()
        .uri(url.as_str())
        .insert_header(header)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::OK);

    // Should parse correctly
    let friendship_history_response: MessageRequestEventResponse =
        test::read_body_json(response).await;

    let message = &friendship_history_response.messages_req_events[0].message;

    assert_eq!("hi, wanna be friends?", message);
    assert_eq!(friendship_history_response.messages_req_events.len(), 1)
}
