#[cfg(test)]
mod tests {

    use crate::helpers::server::{get_app, get_configuration};
    use actix_web::{body::MessageBody, test};
    use social_service::routes::v1::friendships::types::FriendshipsResponse;

    #[actix_web::test]
    async fn test_get_friends() {
        let config = get_configuration();

        let app = test::init_service(get_app(config, None).await).await;
        let token = "my token";

        let req = test::TestRequest::get()
            .uri("/v1/friendships/0xa23")
            .append_header(("Authorization", token))
            .to_request();

        let response = test::call_service(&app, req).await;

        // assert!(response.status().is_success());

        // Should parse correctly
        // let _friendships_response: FriendshipsResponse = test::read_body_json(response).await;
    }

    
}
