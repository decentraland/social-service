mod common;
pub use common::*;

use actix_web::test;

// const METRICS_URI: &str = "/metrics";

// #[actix_web::test]
// async fn metrics_endpoint_should_work() {
//     let mut config = get_configuration().await;
//     config.wkc_metrics_bearer_token = String::from("TEST_TOKEN");
//     let token = config.wkc_metrics_bearer_token.clone();

//     let app = test::init_service(get_app(config, None).await).await;

//     let header = ("authorization", format!("Bearer {token}"));

//     let req = test::TestRequest::get()
//         .uri(METRICS_URI)
//         .insert_header(header)
//         .to_request();

//     let response = test::call_service(&app, req).await;

//     assert!(response.status().is_success());
// }

// #[actix_web::test]
// async fn metrics_endpoint_should_fail_401_when_no_token() {
//     let mut config = get_configuration().await;
//     config.wkc_metrics_bearer_token = String::from("TEST_TOKEN");

//     let app = test::init_service(get_app(config, None).await).await;

//     let header = ("authorization", format!("Bearer {}", ""));

//     let req = test::TestRequest::get()
//         .uri(METRICS_URI)
//         .insert_header(header)
//         .to_request();

//     let response = test::call_service(&app, req).await;

//     assert_eq!(response.status(), 401)
// }

// #[actix_web::test]
// async fn metrics_endpoint_should_fail_401_when_no_header() {
//     let mut config = get_configuration().await;
//     config.wkc_metrics_bearer_token = String::from("TEST_TOKEN");

//     let app = test::init_service(get_app(config, None).await).await;

//     let req = test::TestRequest::get().uri(METRICS_URI).to_request();

//     let response = test::call_service(&app, req).await;

//     assert_eq!(response.status(), 401)
// }

// #[actix_web::test]
// async fn metrics_endpoint_should_fail_500() {
//     let config = get_configuration().await;

//     let app = test::init_service(get_app(config, None).await).await;

//     let req = test::TestRequest::get().uri(METRICS_URI).to_request();

//     let response = test::call_service(&app, req).await;

//     assert_eq!(response.status(), 500)
// }
