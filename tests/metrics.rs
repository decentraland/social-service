mod helpers;
#[cfg(test)]
mod metrics_endpoint_tests {

    use crate::helpers::server::{get_app, get_configuration};
    use actix_web::test;

    #[actix_web::test]
    async fn metrics_endpoint_should_work() {
        let mut config = get_configuration();
        config.wkc_metrics_bearer_token = String::from("TEST_TOKEN");
        let token = config.wkc_metrics_bearer_token.clone();

        let app = test::init_service(get_app(config).await).await;

        let uri = format!("/metrics?=bearer_token={}", token);

        let req = test::TestRequest::get().uri(uri.as_str()).to_request();

        let response = test::call_service(&app, req).await;

        assert!(response.status().is_success());
    }

    #[actix_web::test]
    async fn metrics_endpoint_should_fail_400() {
        let mut config = get_configuration();
        config.wkc_metrics_bearer_token = String::from("TEST_TOKEN");

        let app = test::init_service(get_app(config).await).await;

        let req = test::TestRequest::get().uri("/metrics").to_request();

        let response = test::call_service(&app, req).await;

        assert_eq!(response.status(), 400)
    }

    #[actix_web::test]
    async fn metrics_endpoint_should_fail_500() {
        let config = get_configuration();

        let app = test::init_service(get_app(config).await).await;

        let req = test::TestRequest::get().uri("/metrics").to_request();

        let response = test::call_service(&app, req).await;

        assert_eq!(response.status(), 500)
    }
}
