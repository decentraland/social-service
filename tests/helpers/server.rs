use std::collections::HashMap;

use actix_web::{body::MessageBody, dev::ServiceFactory, web::Data, App};
use async_trait::async_trait;
use mockall::mock;
use social_service::{
    components::{
        app::new_app,
        configuration::{Config, Database},
        health::{HealthComponent, Healthy},
        synapse::{SynapseComponent, VersionResponse},
    },
    get_app_data, get_app_router,
    routes::health::handlers::ComponentHealthStatus,
};
use sqlx::{Connection, Executor, PgConnection, PgPool};

mock! {
    #[derive(Debug)]
    pub Health {}

    #[async_trait]
    impl HealthComponent for Health {
        fn register_component(&mut self, component: Box<dyn Healthy + Send + Sync>, name: String) {
            self.components_to_check
            .push(ComponentToCheck { component, name });
        }
        async fn calculate_status(&self) -> HashMap<String, ComponentHealthStatus> {
            HashMap::new()
        }
    }
}

mock! {
    #[derive(Debug)]
    pub Synapse {}

    #[async_trait]
    impl SynapseComponent for Synapse {
        fn new(url: String) -> Self {
            Self {
                url
            }
        }
        async fn get_version(&self) -> Result<VersionResponse, String> {
            Ok(VersionResponse{
                versions: Vec::new(),
                unstable_features: HashMap::new()
            })
        }
    }

}

pub async fn get_testing_app(
    config: Config,
) -> App<
    impl ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    create_test_db(&config.db).await;
    let app_data = new_app::<MockHealth, MockSynapse>(Some(config)).await;
    get_app_router(&Data::new(app_data))
}

pub fn get_configuration() -> Config {
    let mut config = Config::new().expect("Couldn't read the configuration file");
    config.db.name = uuid::Uuid::new_v4().to_string();
    config
}

pub async fn get_app(
    config: Config,
) -> App<
    impl ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    create_test_db(&config.db).await;
    let app_data = get_app_data(Some(config)).await;
    get_app_router(&app_data)
}

/// We need this to avoid conccurency issues in Tests
pub async fn create_test_db(db_config: &Database) -> PgPool {
    let url = format!(
        "postgres://{}:{}@{}",
        db_config.user, db_config.password, db_config.host
    );

    let mut connection = PgConnection::connect(url.as_str())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, db_config.name).as_str())
        .await
        .expect("Failed to create database");

    let url = format!(
        "postgres://{}:{}@{}/{}",
        db_config.user, db_config.password, db_config.host, db_config.name
    );

    let pool = PgPool::connect(&url)
        .await
        .expect("Failed to connect to Postgress.");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate DB");

    pool
}
