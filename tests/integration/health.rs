#[cfg(test)]
mod tests {

    use crate::helpers::server::start_server;

    #[actix_web::test]
    async fn test_index_get() {
        let _ = start_server().await;
        let client = reqwest::Client::new();

        // Act
        let response = client
            // Use the returned application address
            .get(&format!("http://0.0.0.0:3010/health"))
            .send()
            .await;

        match response {
            Ok(response) => {
                assert!(response.status().is_success());
                assert_ne!(Some(0), response.content_length());
            }
            Err(error) => log::error!("Error querying health endpoint {}", error),
        }
    }
}
