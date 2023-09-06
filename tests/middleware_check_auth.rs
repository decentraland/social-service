mod common;
use std::collections::HashMap;

pub use common::*;

use actix_web::{
    test,
    web::{self, Data},
    App, HttpMessage, HttpResponse,
};
use social_service::{
    api::middlewares::check_auth::CheckAuthToken,
    components::{app::AppComponents, users_cache::UserId},
};

#[actix_web::test]
async fn should_fail_without_authorization_header() {
    let synapse_server = create_synapse_mock_server().await;
    let mut config = get_configuration().await;
    config.synapse.url = synapse_server.uri();

    let app = test::init_service(get_app(config, None).await).await;

    let req = actix_web::test::TestRequest::get()
        .uri("/v1/friendships/0xa")
        .to_request();

    let resp = actix_web::test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400)
}

#[actix_web::test]
async fn should_not_call_synapse_when_token_available_in_redis() {
    let user_id = "0xa";
    let token = "thisisatoken";

    let synapse_server = mock_server_expect_no_calls().await;

    let mut config = get_configuration().await;

    config.synapse.url = synapse_server.uri();

    let components = AppComponents::new(Some(config.clone())).await;
    components
        .users_cache
        .lock()
        .await
        .add_user(token, user_id, user_id, None)
        .await
        .expect("can add user");

    let app_data = Data::new(components);
    let opts = vec!["/need-auth".to_string()];
    // unit app to unit test middleware
    let app = actix_web::test::init_service(
        App::new()
            .app_data(app_data)
            .wrap(CheckAuthToken::new(opts))
            .route("/need-auth", web::get().to(HttpResponse::Ok)),
    )
    .await;
    let header = ("authorization", format!("Bearer {token}"));

    let req = actix_web::test::TestRequest::get()
        .uri("/need-auth")
        .insert_header(header)
        .to_request();

    let resp = actix_web::test::call_service(&app, req).await;

    let ctx_user_id = resp
        .request()
        .extensions()
        .get::<UserId>()
        .map(|u| u.social_id.clone());

    let status = resp.status();

    assert!(ctx_user_id.is_some());
    assert_eq!(ctx_user_id.unwrap(), user_id);
    assert_eq!(status, 200);
}

#[actix_web::test]
async fn should_call_synapse_when_token_not_available_in_redis_and_store_a_clean_user_id_into_redis(
) {
    let user_id_synapse = "@0xb:decentraland.org";
    let token = "a_random_token_";

    let mut token_to_user_id: HashMap<String, String> = HashMap::new();
    token_to_user_id.insert(token.to_string(), user_id_synapse.to_string());

    let synapse_server = who_am_i_synapse_mock_server(token_to_user_id).await;
    let mut config = get_configuration().await;
    config.synapse.url = synapse_server.uri();

    let app = test::init_service(get_app(config, None).await).await;

    let header = ("authorization", format!("Bearer {token}"));

    let req = actix_web::test::TestRequest::get()
        .uri("/v1/friendships/0xb")
        .insert_header(header)
        .to_request();

    let resp = actix_web::test::call_service(&app, req).await;
    let extensions = resp.request().extensions();
    let ctx_user_id = extensions.get::<UserId>();
    assert_eq!(resp.status(), 200);
    assert!(ctx_user_id.is_some());
    // Check that the id sent to synapse has the matrix format
    assert_eq!(ctx_user_id.unwrap().synapse_id, user_id_synapse)
}
