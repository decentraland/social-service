#[cfg(test)]
mod tests {

    use crate::helpers::server::{get_app, get_configuration};
    use actix_web::test;
    use faux::when;
    use social_service::{
        components::{
            app::CustomComponents,
            synapse::{SynapseComponent, WhoAmIResponse},
        },
        routes::v1::friendships::types::FriendshipsResponse,
    };

    #[actix_web::test]
    async fn test_get_friends() {
        let user_id = "a-test-id";
        let config = get_configuration();
        let mut mocked_synapse = SynapseComponent::faux();

        when!(mocked_synapse.who_am_i).then(|_| {
            Ok(WhoAmIResponse {
                user_id: user_id.to_string(),
            })
        });
        when!(mocked_synapse.who_am_i).once();

        let mocked_components = CustomComponents {
            synapse: Some(mocked_synapse),
            db: None,
            redis: None,
            users_cache: None,
        };

        let app = test::init_service(get_app(config, Some(mocked_components)).await).await;
        let token = "Bearer my-token";

        let url = format!("/v1/friendships/{user_id}");

        let req = test::TestRequest::get()
            .uri(url.as_str())
            .append_header(("Authorization", token))
            .to_request();

        let response = test::call_service(&app, req).await;

        assert!(response.status().is_success());

        // Should parse correctly
        let _friendships_response: FriendshipsResponse = test::read_body_json(response).await;
    }
}
