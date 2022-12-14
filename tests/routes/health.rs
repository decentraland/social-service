#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use crate::helpers::server::{
        get_configuration, get_testing_app_data, MockHealth, OptionalComponents,
    };
    use actix_web::test;
    use social_service::{components::synapse::Synapse, get_app_router};

    #[actix_web::test]
    async fn test_index_get() {
        let config = get_configuration();

        // Mocking Example for Health component
        let mut health = MockHealth::default();

        health.expect_register_component().times(2).return_const(());
        health
            .expect_calculate_status()
            .once()
            .returning(|| HashMap::new());

        let components = OptionalComponents {
            health: Some(health),
            synapse: None,
        };

        let app_data = get_testing_app_data::<MockHealth, Synapse>(config, components).await;

        let app = get_app_router(&app_data);

        let app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/health/ready").to_request();

        let response = test::call_service(&app, req).await;

        assert!(response.status().is_success());
    }
}
