use std::collections::HashMap;

use actix_http::StatusCode;
use actix_web::{test, web::Data};
use dcl_http_prom_metrics::HttpMetricsCollectorBuilder;
use social_service::{
    api::{app::get_app_router, routes::v1::friendships::types::FriendshipsResponse},
    components::{app::AppComponents, database::DatabaseComponentImplementation},
};

use super::utils::add_friendship;
use crate::common::*;

// Get friendships/me should return list of friends
#[actix_web::test]
async fn test_get_friendships_me_when_active() {
    let user_id = "a_User_id";
    let other_user_id = "other_useR_id";

    let token = "my-token";

    let mut token_to_user_id: HashMap<String, String> = HashMap::new();
    token_to_user_id.insert(token.to_string(), user_id.to_string());

    let mock_server = who_am_i_synapse_mock_server(token_to_user_id).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let http_metrics_collector = Data::new(HttpMetricsCollectorBuilder::default().build());

    let router = get_app_router(&app_data, &http_metrics_collector);

    let app = test::init_service(router).await;

    add_friendship(&app_data.db, (user_id, other_user_id), true).await;

    let url = "/v1/friendships/me".to_string();

    let header = ("authorization", format!("Bearer {token}"));
    let req = test::TestRequest::get()
        .uri(url.as_str())
        .append_header(header)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::OK);

    // Should parse correctly
    let friendships_response: FriendshipsResponse = test::read_body_json(response).await;
    let friend_address = &friendships_response
        .friendships
        .first()
        .expect("at least one friend")
        .address;
    assert_eq!(friend_address, other_user_id);
}

// Get friends should return list of friends
#[actix_web::test]
async fn test_get_friends_when_active() {
    let user_id = "a_User_id";
    let other_user_id = "other_useR_id";

    let token = "my-token";

    let mut token_to_user_id: HashMap<String, String> = HashMap::new();
    token_to_user_id.insert(token.to_string(), user_id.to_string());

    let mock_server = who_am_i_synapse_mock_server(token_to_user_id).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let http_metrics_collector = Data::new(HttpMetricsCollectorBuilder::default().build());

    let router = get_app_router(&app_data, &http_metrics_collector);

    let app = test::init_service(router).await;

    add_friendship(&app_data.db, (user_id, other_user_id), true).await;

    let url = "/v1/friendships/a_user_Id/".to_string();

    let header = ("authorization", format!("Bearer {token}"));
    let req = test::TestRequest::get()
        .uri(url.as_str())
        .append_header(header)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::OK);

    // Should parse correctly
    let friendships_response: FriendshipsResponse = test::read_body_json(response).await;
    let friend_address = &friendships_response
        .friendships
        .first()
        .expect("at least one friend")
        .address;
    assert_eq!(friend_address, other_user_id);
}

// Get friends should return empty when non-active
#[actix_web::test]
async fn test_get_friends_when_inactive() {
    let user_id = "a_User_id";
    let other_user_id = "other_useR_id";

    let token = "my-token";

    let mut token_to_user_id: HashMap<String, String> = HashMap::new();
    token_to_user_id.insert(token.to_string(), user_id.to_string());

    let mock_server = who_am_i_synapse_mock_server(token_to_user_id).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let http_metrics_collector = Data::new(HttpMetricsCollectorBuilder::default().build());

    let router = get_app_router(&app_data, &http_metrics_collector);

    let app = test::init_service(router).await;

    add_friendship(&app_data.db, (user_id, other_user_id), false).await;

    let url = "/v1/friendships/a_user_Id".to_string();

    let header = ("authorization", format!("Bearer {token}"));
    let req = test::TestRequest::get()
        .uri(url.as_str())
        .append_header(header)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::OK);

    // Should parse correctly
    let friendships_response: FriendshipsResponse = test::read_body_json(response).await;
    assert!(&friendships_response.friendships.is_empty());
}

#[actix_web::test]
async fn should_return_forbidden_when_requester_asks_for_different_user() {
    let user_id = "a_user_id";

    let mut token_to_user_id: HashMap<String, String> = HashMap::new();
    let token = "my-token";
    token_to_user_id.insert(token.to_string(), user_id.to_string());

    let mock_server = who_am_i_synapse_mock_server(token_to_user_id).await;

    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let components = AppComponents::new(Some(config.clone())).await;

    let app = test::init_service(get_app(config, Some(components)).await).await;

    let url = "/v1/friendships/other_user_id";

    let header = ("authorization", format!("Bearer {token}"));
    let req = test::TestRequest::get()
        .uri(url)
        .append_header(header)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn test_get_user_friends_database_error_should_return_unknown_error() {
    let user_id = "a_user_id";
    let mut token_to_user_id: HashMap<String, String> = HashMap::new();
    let token = "my-token";

    token_to_user_id.insert(token.to_string(), user_id.to_string());

    let mock_server = who_am_i_synapse_mock_server(token_to_user_id).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let http_metrics_collector = Data::new(HttpMetricsCollectorBuilder::default().build());

    let router = get_app_router(&app_data, &http_metrics_collector);

    app_data.db.close().await;
    let app = test::init_service(router).await;

    let url = format!("/v1/friendships/{user_id}");

    let header = ("Authorization", format!("Bearer {token}"));
    let req = test::TestRequest::get()
        .uri(url.as_str())
        .append_header(header)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[actix_web::test]
async fn test_get_user_friends_should_return_the_address_list() {
    let user_id = "a_uSer_id";
    let other_user = "b_another_Id";
    let other_user_2 = "b_another_Id_2";

    let mut token_to_user_id: HashMap<String, String> = HashMap::new();
    let token = "my-token";

    token_to_user_id.insert(token.to_string(), user_id.to_string());

    let mock_server = who_am_i_synapse_mock_server(token_to_user_id).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let http_metrics_collector = Data::new(HttpMetricsCollectorBuilder::default().build());

    let router = get_app_router(&app_data, &http_metrics_collector);

    let app = test::init_service(router).await;

    add_friendship(&app_data.db, (user_id, other_user), true).await;
    add_friendship(&app_data.db, (user_id, other_user_2), true).await;

    let url = "/v1/friendships/a_uSer_ID".to_string();

    let header = ("authorization", format!("Bearer {token}"));
    let req = test::TestRequest::get()
        .uri(url.as_str())
        .append_header(header)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::OK);

    // Should parse correctly
    let friendships_response: FriendshipsResponse = test::read_body_json(response).await;
    let addresses: Vec<&str> = friendships_response
        .friendships
        .iter()
        .map(|friendship| friendship.address.as_str())
        .collect();
    assert!(addresses.contains(&other_user));
    assert!(addresses.contains(&other_user_2));
}
