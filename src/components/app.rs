use std::sync::Arc;

use tokio::sync::Mutex;

use super::configuration::Database;
use super::{
    configuration::Config, database::DatabaseComponent, database::DatabaseComponentImplementation,
    health::HealthComponent, synapse::SynapseComponent,
};

use super::{
    redis::Redis,
    users_cache::{self, UsersCacheComponent},
};

pub struct AppComponents {
    pub health: HealthComponent,
    pub synapse: SynapseComponent,
    pub config: Config,
    pub db: DatabaseComponent,
    pub users_cache: Arc<Mutex<UsersCacheComponent>>,
}

impl AppComponents {
    /// Panics if there is no config provided and cannot bulid a new config
    pub async fn new(custom_config: Option<Config>) -> Self {
        let config = custom_config
            .unwrap_or_else(|| Config::new().expect("Couldn't read the configuration"));

        Self::with_config(config).await
    }

    async fn with_config(config: Config) -> Self {
        if env_logger::try_init().is_err() {
            log::debug!("Logger already init")
        }

        let synapse = Self::init_synapse_component(config.synapse.url.clone());
        let db = Self::init_db_component(&config.db).await;
        let redis = Redis::new_and_run(&config.redis).await;
        match redis {
            Ok(redis) => {
                let health = Self::init_health_component(db.clone(), redis.clone());
                let users_cache = Self::init_users_cache(redis, config.cache_hashing_key.clone());

                Self {
                    health,
                    db,
                    synapse,
                    users_cache: Arc::new(Mutex::new(users_cache)),
                    config,
                }
            }
            Err(er) => {
                log::error!("There was an error initiliazing Redis: {}", er);
                panic!("There was an error initializing Redis");
            }
        }
    }

    fn init_health_component(db: DatabaseComponent, redis: Redis) -> HealthComponent {
        let mut health = HealthComponent::default();
        health.register_component(Box::new(db), "database".to_string());
        health.register_component(Box::new(redis), "redis".to_string());
        health
    }

    async fn init_db_component(db_config: &Database) -> DatabaseComponent {
        let mut db = DatabaseComponent::new(db_config);
        if let Err(err) = db.run().await {
            log::debug!("Error on running the DB: {:?}", err);
            panic!("Unable to run the DB")
        }
        db
    }

    fn init_synapse_component(url: String) -> SynapseComponent {
        SynapseComponent::new(url)
    }

    fn init_users_cache(redis: Redis, config_hash_key: String) -> UsersCacheComponent {
        users_cache::UsersCacheComponent::new(redis, config_hash_key)
    }
}
