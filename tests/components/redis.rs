#[cfg(test)]
mod tests {

    use crate::helpers::server::{get_app, get_configuration};
    use actix_web::test;

    #[actix_web::test]
    async fn test_can_get_redis_connection() {
        let config = get_configuration();

        let app = get_app(config).await;

        let service = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/health/ready").to_request();

        let response = test::call_service(&service, req).await;

        assert!(response.status().is_success())
    }
}
