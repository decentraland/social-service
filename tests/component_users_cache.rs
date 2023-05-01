use std::time::Duration;

use social_service::components::{
    configuration::RedisConfig,
    redis::Redis,
    users_cache::{UserId, UsersCacheComponent},
};

use actix_rt::time::sleep;

const TEST_KEY: &str = "TEST KEY";

async fn create_users_cache_component() -> UsersCacheComponent {
    let redis = Redis::new_and_run(&RedisConfig {
        host: "0.0.0.0:6379".to_string(),
    })
    .await
    .expect("There was an error initializing Redis");

    UsersCacheComponent::new(redis, TEST_KEY.to_string())
}

#[actix_web::test]
async fn test_should_return_no_connection_available() -> Result<(), String> {
    let token = "my test token";
    let user_id = "joni";

    let redis = Redis::new_and_run(&RedisConfig {
        host: "0.0.0.0:6379".to_string(),
    })
    .await
    .expect("Failed starting Redis");

    // When redis is closed, adding a user should return an error
    redis.stop();

    let mut user_cache_component = UsersCacheComponent::new(redis, TEST_KEY.to_string());
    let res = user_cache_component
        .add_user(token, user_id, user_id, None)
        .await;

    match res {
        Ok(_) => Err("Should return the expected error".to_string()),
        Err(err) => {
            assert_eq!(
                format!("Couldn't cache user {user_id}, redis has no connection available"),
                err
            );
            Ok(())
        }
    }
}

#[actix_web::test]
async fn test_can_store_and_get_user() {
    let mut component = create_users_cache_component().await;

    let user_id = "my user id";
    let token = "an example token";

    let store = component.add_user(token, user_id, user_id, None).await;

    if let Err(err) = store {
        panic!("Couldn't store the user {user_id} due to {err}");
    }

    let res = component.get_user(token).await;

    match res {
        Ok(_) => assert_eq!(
            res.unwrap(),
            UserId {
                social_id: user_id.to_string(),
                synapse_id: user_id.to_string()
            }
        ),
        Err(err) => {
            panic!("Couldn't get the user {user_id} due to {err}")
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
        .add_user(token, user_id, user_id, Some(expiring_time))
        .await;

    if let Err(err) = store {
        panic!("Couldn't store the user {user_id} due to {err}");
    }

    // wait for the key to expire
    sleep(Duration::from_secs(2)).await;

    let res = component.get_user(token).await;

    match res {
        Ok(_) => panic!("Got the user {user_id}"),
        Err(err) => {
            assert!(err.contains("(response was nil)"))
        }
    }
}
