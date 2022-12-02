mod helpers;
#[cfg(test)]
mod tests {

    use crate::helpers::server::{get_app, get_configuration};
    use actix_web::test;

    #[actix_web::test]
    async fn test_index_get() {
        let config = get_configuration();

        let app = test::init_service(get_app(config).await).await;

        let req = test::TestRequest::get().uri("/health/ready").to_request();

        let response = test::call_service(&app, req).await;

        assert!(response.status().is_success())
    }
}
