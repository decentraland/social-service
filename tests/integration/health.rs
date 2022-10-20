#[cfg(test)]
mod tests {
    use actix_web::{
        dev::Service,
        http::header::ContentType,
        test,
        web::{self, Data},
        App,
    };
    use social_service::{components::app::AppComponents, routes::health::health::health};

    #[actix_web::test]
    async fn test_index_get() {
        let service = build_app().await;
        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .uri("/health")
            .to_request();
        let resp = test::call_service(&service, req).await;

        println!("{}", resp.status());
        assert!(resp.status().is_success());
    }
}
