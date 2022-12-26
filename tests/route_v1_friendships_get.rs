mod common;

use actix_web::{test, web::Data};
use reqwest::StatusCode;
use social_service::{
    components::app::AppComponents, get_app_router,
    routes::v1::friendships::types::FriendshipsResponse,
};

use common::{get_app, get_configuration, who_am_i_synapse_mock_server};

// Get friends should return list of friends
#[actix_web::test]
async fn test_get_friends() {
    let user_id = "a_user_id";
    let other_user_id = "other_user_id";

    let mock_server = who_am_i_synapse_mock_server(user_id.to_string()).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let router = get_app_router(&app_data);

    let app = test::init_service(router).await;

    app_data
        .db
        .db_repos
        .as_ref()
        .expect("repos to be present")
        .friendships
        .create_new_friendships((user_id, other_user_id))
        .await
        .expect("can create friendship");

    let token = "my-token";

    let url = format!("/v1/friendships/{user_id}");

    let header = ("authorization", format!("Bearer {}", token));
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

#[actix_web::test]
async fn should_return_forbidden_when_requester_asks_for_different_user() {
    let user_id = "a_user_id";

    let mock_server = who_am_i_synapse_mock_server(user_id.to_string()).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let components = AppComponents::new(Some(config.clone())).await;

    let app = test::init_service(get_app(config, Some(components)).await).await;

    let token = "my-token";

    let url = "/v1/friendships/other_user_id";

    let header = ("authorization", format!("Bearer {}", token));
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
    let mock_server = who_am_i_synapse_mock_server(user_id.to_string()).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let router = get_app_router(&app_data);

    let app = test::init_service(router).await;
    app_data.db.close().await;

    let token = "my-token";

    let url = format!("/v1/friendships/{user_id}");

    let header = ("Authorization", format!("Bearer {}", token));
    let req = test::TestRequest::get()
        .uri(url.as_str())
        .append_header(header)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[actix_web::test]
async fn test_get_user_friends_should_return_the_address_list() {
    let user_id = "custom id";
    let other_user = "another id";
    let other_user_2 = "another id 2";
    // let app_data = Data::new(AppComponents::new(Some(cfg)).await);
}
