use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use actix_http::StatusCode;
use actix_web::test;
use actix_web::web::Data;

use social_service::get_app_router;
use social_service::routes::v1::friendships::history::RequestEventRequestBody;
use social_service::routes::v1::friendships::types::MessageRequestEventResponse;
use sqlx::types::Json;
use uuid::uuid;

use social_service::components::app::AppComponents;

use crate::common::*;
use crate::routes::v1::friendships::utils::create_friendship_history;

#[actix_rt::test]
async fn test_get_sent_messages_request_event() {
    let user_id = "a_user_id";
    let friendship_id = uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8");

    let room_message_body = Some("Hola");
    let metadata = room_message_body.map(|body| {
        let mut data = HashMap::new();
        data.insert("message_body".to_string(), body.to_string());
        Json(data)
    });
    let metadata_other_key = room_message_body.map(|body| {
        let mut data = HashMap::new();
        data.insert("other_key".to_string(), body.to_string());
        Json(data)
    });

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

    // Create friendship request entry with metadata
    // that contains the key `message_body`
    create_friendship_history(
        &app_data.db,
        friendship_id,
        "\"request\"",
        user_id,
        metadata,
    )
    .await;
    // Create friendship request entry with metadata
    // that does not contain the key `message_body`
    create_friendship_history(
        &app_data.db,
        friendship_id,
        "\"request\"",
        user_id,
        metadata_other_key,
    )
    .await;
    // Create friendship request entry without metadata
    create_friendship_history(&app_data.db, friendship_id, "\"request\"", user_id, None).await;

    let url = format!("/v1/friendships/{friendship_id}/request-events/messages");

    let header = ("authorization", format!("Bearer {}", token));
    let req = test::TestRequest::get()
        .uri(url.as_str())
        .set_json(&RequestEventRequestBody {
            timestamp_from: 1662921288, // Sunday, September 11, 2022 6:34:48 PMs
            timestamp_to: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        })
        .insert_header(header)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::OK);

    // Should parse correctly
    let friendship_history_response: MessageRequestEventResponse =
        test::read_body_json(response).await;

    let message = &friendship_history_response.messages_req_events[0].body;

    assert_eq!("Hola", message);
    assert_eq!(friendship_history_response.messages_req_events.len(), 1)
}