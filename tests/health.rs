mod helpers;
#[cfg(test)]
mod tests {

    use crate::helpers::server::{get_configuration, start_server};

    #[actix_web::test]
    async fn test_index_get() {
        let config = get_configuration();
        let _ = start_server(config.clone()).await;
        let client = reqwest::Client::new();

        // Act
        let response = client
            // Use the returned application address
            .get(&format!(
                "http://{}:{}/health",
                config.server.host,
                config.server.port.to_string()
            ))
            .send()
            .await;

        match response {
            Ok(response) => {
                assert!(response.status().is_success());
                assert_ne!(Some(0), response.content_length());
            }
            Err(error) => {
                panic!("Error querying health endpoint {}", error)
            }
        }
    }
}
