use super::{
    configuration::Config, database::DatabaseComponent, health::HealthComponent,
    synapse::SynapseComponent,
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
    pub user_cache: UsersCacheComponent,
}

impl AppComponents {
    pub async fn new(custom_config: Option<Config>) -> Self {
        if let Err(_) = env_logger::try_init() {
            log::debug!("Logger already init")
        }

        // Initialize components
        let config = custom_config
            .unwrap_or_else(|| Config::new().expect("Couldn't read the configuration"));

        let mut health = HealthComponent::default();
        let synapse = SynapseComponent::new(config.synapse.url.clone());
        let mut db = DatabaseComponent::new(&config.db);

        if let Err(err) = db.run().await {
            log::debug!("Error on running the DB: {:?}", err);
            panic!("Unable to run the DB")
        }

        let redis = Redis::new_and_run(&config.redis);

        // TODO: Should we refactor HealthComponent to avoid cloning structs?
        health.register_component(Box::new(db.clone()), "database".to_string());
        health.register_component(Box::new(redis.clone()), "redis".to_string());
        let users_cache_instance =
            users_cache::UsersCacheComponent::new(redis, config.cache_hashing_key.clone());

        Self {
            config,
            health,
            synapse,
            db,
            user_cache: users_cache_instance,
        }
    }
}
