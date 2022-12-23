use actix_web::{test, web::Data};
use social_service::components::synapse::{WhoAmIResponse, WHO_AM_I_URI};
use social_service::{
    components::app::AppComponents, routes::v1::friendships::types::FriendshipsResponse,
};

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::helpers::server::{get_app, get_configuration};

/// Start a background HTTP server on a random local port mocking Who AM I endpoint.
async fn who_am_i_synapse(user_id: String) -> MockServer {
    let mock_server = MockServer::start().await;

    let who_am_i = WhoAmIResponse { user_id };
    Mock::given(method("GET"))
        .and(path(WHO_AM_I_URI))
        .respond_with(ResponseTemplate::new(200).set_body_json(who_am_i))
        .mount(&mock_server)
        .await;

    mock_server
}

// Get friends should return list of friends
#[actix_web::test]
async fn test_get_friends() {
    let user_id = "a-test-id";

    let mock_server = who_am_i_synapse(user_id.to_string()).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let components = AppComponents::new(Some(config.clone())).await;

    let app = test::init_service(get_app(config, Some(components)).await).await;

    let token = "Bearer my-token";

    let url = format!("/v1/friendships/{user_id}");

    let req = test::TestRequest::get()
        .uri(url.as_str())
        .append_header(("Authorization", token))
        .to_request();

    let response = test::call_service(&app, req).await;

    assert!(response.status().is_success());

    // Should parse correctly
    let _friendships_response: FriendshipsResponse = test::read_body_json(response).await;
}

#[actix_web::test]
async fn should_return_forbidden_when_requester_asks_for_different_user() {
    let user_id = "a-test-id";
    let mut config = get_configuration().await;
    config.synapse.url = "mocked".to_string(); // TODO ask for mocked server uri

    let app = test::init_service(get_app(config, None).await).await;
}

#[actix_web::test]
async fn test_get_user_friends_database_error_should_return_unknown_error() {
    // let app_data = Data::new(AppComponents::new(Some(cfg)).await);
}

#[actix_web::test]
async fn test_get_user_friends_should_return_the_address_list() {
    let user_id = "custom id";
    let other_user = "another id";
    let other_user_2 = "another id 2";
    // let app_data = Data::new(AppComponents::new(Some(cfg)).await);
}
