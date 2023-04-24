use deadpool_redis::redis::cmd;
use social_service::components::{configuration::RedisConfig, redis::Redis};

async fn create_redis_component() -> Redis {
    Redis::new_and_run(&RedisConfig {
        host: "0.0.0.0:6379".to_string(),
    })
    .await
    .expect("There was an error initializing Redis")
}

#[actix_web::test]
async fn test_can_get_redis_connection() {
    let component = create_redis_component().await;
    let con = component.get_async_connection().await;

    if con.is_none() {
        panic!("Failed creating connection with Redis");
    }
}

#[actix_web::test]
async fn test_can_store_key_in_redis() {
    let component = create_redis_component().await;

    let key = "my_key";
    let value = "value";

    {
        let mut connection = component.get_async_connection().await.unwrap();
        cmd("SET")
            .arg(&[key, value])
            .query_async::<_, ()>(&mut connection)
            .await
            .unwrap();
    }

    {
        let mut connection = component.get_async_connection().await.unwrap();
        let res_value: String = cmd("GET")
            .arg(&[key])
            .query_async(&mut connection)
            .await
            .unwrap();

        assert_eq!(res_value, value);
    }
}
