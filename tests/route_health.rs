mod common;
pub use common::*;

use actix_web::test;
use common::{get_app, get_configuration};

#[actix_web::test]
async fn test_index_get() {
    let config = get_configuration().await;

    let app = test::init_service(get_app(config, None).await).await;

    let req = test::TestRequest::get().uri("/health/ready").to_request();

    let response = test::call_service(&app, req).await;

    assert!(response.status().is_success())
}
