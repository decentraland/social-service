#[cfg(test)]
mod check_auth_middleware_tests {
    use actix_web::{test, web, App, HttpResponse};
    use mockall::mock;
    use social_service::{
        components::synapse::{SynapseComponent, VersionResponse, WhoAmIResponse},
        middlewares::check_auth::CheckAuthToken,
    };

    mock! {
        #[derive(Debug)]
        Synapse {}

        #[async_trait::async_trait]
        impl SynapseComponent for Synapse {
            async fn get_version(&self) -> Result<VersionResponse, String> {
                Ok(VersionResponse{})
            }
            async fn who_am_i(&self, token: &str) -> Result<WhoAmIResponse, String> {
                Ok(WhoAmIResponse{})
            }
        }

    }

    #[actix_web::test]
    async fn should_fail_without_authorization_header() {
        let routes = vec![String::from("/need-auth")];
        let app = test::init_service(
            App::new()
                .wrap(CheckAuthToken::new(routes))
                .route("/need-auth", web::get().to(|| HttpResponse::Accepted())),
        )
        .await;
        let req = test::TestRequest::get().uri("/need-auth").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400)
    }
}
