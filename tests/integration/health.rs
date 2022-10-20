#[cfg(test)]
mod tests {
    use actix_web::rt::task::JoinHandle;

    use social_service::run_service;

    async fn start_server() -> JoinHandle<Result<(), std::io::Error>> {
        let server = run_service().await;

        if let Ok(server) = server {
            actix_web::rt::spawn(server)
        } else {
            panic!("Couldn't run the server");
        }
    }

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
            Err(error) => log::error!("Error {}", error),
        }

        // Assert
    }
}
