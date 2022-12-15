#[cfg(test)]
mod tests {

    use std::time::Duration;

    use social_service::components::{
        configuration::Redis as RedisConfig, redis::Redis, users_cache::UsersCacheComponent,
    };

    use actix_rt::time::sleep;

    const TEST_KEY: &str = "TEST KEY";

    async fn create_users_cache_component() -> UsersCacheComponent {
        let redis = Redis::new_and_run(&RedisConfig {
            host: "0.0.0.0:6379".to_string(),
        });

        UsersCacheComponent::new(redis, TEST_KEY.to_string())
    }

    #[actix_web::test]
    async fn test_can_store_and_get_user() {
        let mut component = create_users_cache_component().await;

        let user_id = "my user id";
        let token = "an example token";

        let store = component.add_user(token, user_id, None).await;

        if let Err(err) = store {
            panic!("Couldn't store the user {} due to {}", user_id, err);
        }

        let res = component.get_user(token).await;

        match res {
            Ok(_) => assert_eq!(res.unwrap(), user_id),
            Err(err) => {
                panic!("Couldn't get the user {} due to {}", user_id, err)
            }
        }
    }

    #[actix_web::test]
    async fn test_obtain_expired_key_returns_none() {
        let mut component = create_users_cache_component().await;

        let user_id = "my expiring id";
        let token = "an example token that expires";
        let expiring_time = 1;

        let store = component
            .add_user(token, user_id, Some(expiring_time))
            .await;

        if let Err(err) = store {
            panic!("Couldn't store the user {} due to {}", user_id, err);
        }

        // wait for the key to expire
        sleep(Duration::from_secs(2)).await;

        let res = component.get_user(token).await;

        match res {
            Ok(_) => panic!("Got the user {}", user_id),
            Err(err) => {
                assert!(err.contains("(response was nil)"))
            }
        }
    }
}