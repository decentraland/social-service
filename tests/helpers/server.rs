use std::collections::HashMap;

use actix_web::{body::MessageBody, dev::ServiceFactory, web::Data, App};
use async_trait::async_trait;
use mockall::automock;
use social_service::{
    components::{
        app::AppComponents,
        configuration::{Config, Database},
        database::DatabaseComponent,
        health::{ComponentToCheck, HealthComponent, Healthy},
        redis::{Redis, RedisComponent},
        synapse::{SynapseComponent, VersionResponse},
        users_cache,
    },
    get_app_data, get_app_router,
    routes::health::handlers::ComponentHealthStatus,
    AppData,
};
use sqlx::{Connection, Executor, PgConnection, PgPool};

pub struct OptionalComponents<H: HealthComponent, S: SynapseComponent> {
    pub health: Option<H>,
    pub synapse: Option<S>,
}
#[derive(Debug)]
pub struct Health {
    components_to_check: Vec<ComponentToCheck>,
}

#[automock]
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

#[derive(Debug)]
pub struct Synapse {
    pub synapse_url: String,
}

#[automock]
#[async_trait]
impl SynapseComponent for Synapse {
    fn new(url: String) -> Self {
        Self { synapse_url: url }
    }
    async fn get_version(&self) -> Result<VersionResponse, String> {
        Ok(VersionResponse {
            versions: Vec::new(),
            unstable_features: HashMap::new(),
        })
    }
}

pub async fn get_testing_app_data<
    H: HealthComponent + Default + Send + Sync + 'static,
    S: SynapseComponent + Send + Sync + 'static,
>(
    config: Config,
    components: OptionalComponents<H, S>,
) -> AppData<H, S> {
    create_test_db(&config.db).await;
    Data::new(new_testing_app(Some(config), components).await)
}

pub async fn new_testing_app<
    H: HealthComponent + Default + Send + Sync + 'static,
    S: SynapseComponent + Send + Sync + 'static,
>(
    custom_config: Option<Config>,
    components: OptionalComponents<H, S>,
) -> AppComponents<H, S> {
    let config =
        custom_config.unwrap_or_else(|| Config::new().expect("Couldn't read the configuration"));

    let mut health = H::default();
    if let Some(h) = components.health {
        health = h
    }

    let mut synapse = S::new(config.synapse.url.clone());
    if let Some(s) = components.synapse {
        synapse = s
    }

    let mut db = DatabaseComponent::new(&config.db);
    let mut redis = Redis::new(&config.redis);

    if let Err(err) = db.run().await {
        log::debug!("Error on running the DB: {:?}", err);
        panic!("Unable to run the DB")
    }

    if let Err(err) = redis.run().await {
        log::debug!("Error while connecting to redis: {:?}", err);
        panic!("Unable connecting to redis {:?}", err)
    }

    // TODO: Should we refactor HealthComponent to avoid cloning structs?
    health.register_component(Box::new(db.clone()), "database".to_string());
    health.register_component(Box::new(redis.clone()), "redis".to_string());
    let users_cache_instance =
        users_cache::UsersCacheComponent::new(redis, config.cache_hashing_key.clone());

    AppComponents::new(health, synapse, config, db, users_cache_instance)
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
