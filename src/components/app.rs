use super::configuration::Database;
use super::{
    configuration::Config, database::DatabaseComponent, health::HealthComponent,
    synapse::SynapseComponent,
};

use super::{
    redis::Redis,
    users_cache::{self, UsersCacheComponent},
};

use std::sync::Mutex;

pub struct AppComponents {
    pub health: HealthComponent,
    pub synapse: SynapseComponent,
    pub config: Config,
    pub db: DatabaseComponent,
    pub users_cache: Mutex<UsersCacheComponent>,
}

pub struct CustomComponents {
    pub synapse: Option<SynapseComponent>,
    pub db: Option<DatabaseComponent>,
    pub users_cache: Option<UsersCacheComponent>,
    pub redis: Option<Redis>,
}

impl AppComponents {
    pub async fn new(
        custom_config: Option<Config>,
        custom_components: Option<CustomComponents>,
    ) -> Self {
        let config = custom_config
            .unwrap_or_else(|| Config::new().expect("Couldn't read the configuration"));
        if custom_components.is_none() {
            AppComponents::default(config).await
        } else {
            // For testing propouses mainly
            let custom = custom_components.unwrap();
            AppComponents::custom(config, custom).await
        }
    }

    async fn default(config: Config) -> Self {
        if let Err(_) = env_logger::try_init() {
            log::debug!("Logger already init")
        }

        // Initialize components
        let synapse = AppComponents::init_synapse_component(config.synapse.url.clone());
        let db = AppComponents::init_db_component(&config.db).await;
        let redis = Redis::new_and_run(&config.redis);

        // TODO: Should we refactor HealthComponent to avoid cloning structs?
        let health = AppComponents::init_health_component(db.clone(), redis.clone());

        let users_cache_instance =
            AppComponents::init_users_cache(redis, config.cache_hashing_key.clone());

        Self {
            config,
            health,
            synapse,
            db,
            users_cache: Mutex::new(users_cache_instance),
        }
    }

    async fn custom(config: Config, custom_components: CustomComponents) -> Self {
        if let Err(_) = env_logger::try_init() {
            log::debug!("Logger already init")
        }

        let mut custom_components = custom_components;

        let synapse: SynapseComponent;

        if custom_components.synapse.is_some() {
            synapse = custom_components.synapse.take().unwrap();
        } else {
            synapse = AppComponents::init_synapse_component(config.synapse.url.clone());
        }

        let db: DatabaseComponent;

        if custom_components.db.is_some() {
            db = custom_components.db.take().unwrap();
        } else {
            db = AppComponents::init_db_component(&config.db).await;
        }

        let redis: Redis;

        if custom_components.redis.is_some() {
            redis = custom_components.redis.take().unwrap();
        } else {
            redis = Redis::new_and_run(&config.redis);
        }

        let health = AppComponents::init_health_component(db.clone(), redis.clone());

        let users_cache: UsersCacheComponent;

        if custom_components.users_cache.is_some() {
            users_cache = custom_components.users_cache.take().unwrap();
        } else {
            users_cache = AppComponents::init_users_cache(redis, config.cache_hashing_key.clone());
        }

        Self {
            health,
            db,
            synapse,
            users_cache: Mutex::new(users_cache),
            config,
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
