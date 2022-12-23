use actix_web::{test, HttpMessage};
use social_service::middlewares::check_auth::UserId;

use crate::helpers::server::{get_app, get_configuration};

#[actix_web::test]
async fn should_fail_without_authorization_header() {
    // TODO mock server and expect
    let mut config = get_configuration().await;
    config.synapse.url = "mocked".to_string(); // TODO ask for mocked server uri
    let app = test::init_service(get_app(config, None).await).await;

    let req = actix_web::test::TestRequest::get()
        .uri("/v1/friendships/0xa")
        .to_request();

    let resp = actix_web::test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400)
}

#[actix_web::test]
async fn should_not_call_synapse_when_token_available_in_redis() {
    // TODO mock server and expect
    let mut config = get_configuration().await;
    config.synapse.url = "mocked".to_string(); // TODO ask for mocked server uri

    let app = test::init_service(get_app(config, None).await).await;

    let token = "this is a token";
    let user_id = "0xa";

    let header = ("authorization", format!("Bearer {}", token));

    let req = actix_web::test::TestRequest::get()
        .uri("/v1/friendships/0x1")
        .insert_header(header)
        .to_request();

    let resp = actix_web::test::call_service(&app, req).await;
    let extensions = resp.request().extensions();
    let ctx_user_id = extensions.get::<UserId>();
    assert_eq!(resp.status(), 200);
    assert!(ctx_user_id.is_some());
    assert_eq!(ctx_user_id.unwrap().0, user_id)
}

#[actix_web::test]
async fn should_call_synapse_when_token_not_available_in_redis_and_store_userid_into_redis() {
    // TODO mock server and expect
    let mut config = get_configuration().await;
    config.synapse.url = "mocked".to_string(); // TODO ask for mocked server uri

    let app = test::init_service(get_app(config, None).await).await;

    let token = "a1b2c3d4";
    let user_id = "0xa";

    // unit app to unit test middleware
    let header = ("authorization", format!("Bearer {}", token));

    // TODO: call to authenticated endpoint
    let req = actix_web::test::TestRequest::get()
        .uri("/need-auth")
        .insert_header(header)
        .to_request();

    let resp = actix_web::test::call_service(&app, req).await;
    let extensions = resp.request().extensions();
    let ctx_user_id = extensions.get::<UserId>();
    assert_eq!(resp.status(), 200);
    assert!(ctx_user_id.is_some());
    assert_eq!(ctx_user_id.unwrap().0, user_id)
}
