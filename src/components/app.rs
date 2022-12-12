use std::sync::Arc;

use super::health::HealthComponent;
use super::synapse::SynapaseComponent;
use super::users_cache::UsersCacheComponent;
use super::{
    configuration::Config, database::DatabaseComponent, health::Health, redis::RedisComponent,
    synapse::Synapse,
};

use super::{
    redis::Redis,
    users_cache::{self, UsersCache},
};

pub trait AppComponents {
    fn get_health_component(&self) -> Arc<dyn HealthComponent>;
    fn get_synapse_component(&self) -> Arc<dyn SynapaseComponent>;
    fn get_users_cache_component(&self) -> Arc<dyn UsersCacheComponent>;
    fn get_config(&self) -> &Config;
}

pub struct App {
    pub health: Arc<Health>,
    pub synapse: Arc<Synapse>,
    pub config: Config,
    pub db: DatabaseComponent,
    pub users_cache: Arc<UsersCache<Redis>>,
}

impl App {
    pub async fn new(custom_config: Option<Config>) -> Arc<dyn AppComponents + Send + Sync> {
        if let Err(_) = env_logger::try_init() {
            log::debug!("Logger already init")
        }

        // Initialize components
        let config = custom_config
            .unwrap_or_else(|| Config::new().expect("Couldn't read the configuration"));

        let mut health = Health::default();
        let synapse = Synapse::new(config.synapse.url.clone());
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
            users_cache::UsersCache::new(redis, config.cache_hashing_key.clone());

        Arc::new(Self {
            config,
            health: Arc::new(health), // in order to be able to mutate above (register_component)
            synapse: Arc::new(synapse),
            db,
            users_cache: Arc::new(users_cache_instance),
        })
    }
}

impl AppComponents for App {
    fn get_health_component(&self) -> Arc<dyn HealthComponent> {
        self.health.clone()
    }
    fn get_config(&self) -> &Config {
        &self.config
    }
    fn get_synapse_component(&self) -> Arc<dyn SynapaseComponent> {
        self.synapse.clone()
    }
    fn get_users_cache_component(&self) -> Arc<dyn UsersCacheComponent> {
        self.users_cache.clone()
    }
}
