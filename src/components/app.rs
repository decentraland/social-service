use futures_util::lock::Mutex;

use super::configuration::Database;
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
    pub users_cache: Mutex<UsersCacheComponent>,
}

pub struct CustomComponents {
    pub synapse: Option<SynapseComponent>,
    pub db: Option<DatabaseComponent>,
    pub users_cache: Option<UsersCacheComponent>,
    pub redis: Option<Redis>,
}

#[derive(Default)]
pub struct CustomComponentsBuilder {
    synapse: Option<SynapseComponent>,
    db: Option<DatabaseComponent>,
    users_cache: Option<UsersCacheComponent>,
    redis: Option<Redis>,
}

impl CustomComponentsBuilder {
    pub fn with_synapse(mut self, synapse: SynapseComponent) -> Self {
        self.synapse = Some(synapse);
        self
    }

    pub fn with_db(mut self, db: DatabaseComponent) -> Self {
        self.db = Some(db);
        self
    }

    pub fn with_users_cache(mut self, users_cache: UsersCacheComponent) -> Self {
        self.users_cache = Some(users_cache);
        self
    }

    pub fn with_redis(mut self, redis: Redis) -> Self {
        self.redis = Some(redis);
        self
    }

    pub fn build(self) -> CustomComponents {
        CustomComponents {
            synapse: self.synapse,
            db: self.db,
            users_cache: self.users_cache,
            redis: self.redis,
        }
    }
}

impl CustomComponents {
    pub fn builder() -> CustomComponentsBuilder {
        CustomComponentsBuilder::default()
    }
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
        if env_logger::try_init().is_err() {
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
        if env_logger::try_init().is_err() {
            log::debug!("Logger already init")
        }

        let mut custom_components = custom_components;

        let synapse: SynapseComponent = if custom_components.synapse.is_some() {
            custom_components.synapse.take().unwrap()
        } else {
            AppComponents::init_synapse_component(config.synapse.url.clone())
        };

        let db: DatabaseComponent = if custom_components.db.is_some() {
            custom_components.db.take().unwrap()
        } else {
            AppComponents::init_db_component(&config.db).await
        };

        let redis: Redis = if custom_components.redis.is_some() {
            custom_components.redis.take().unwrap()
        } else {
            Redis::new_and_run(&config.redis)
        };

        let health = AppComponents::init_health_component(db.clone(), redis.clone());

        let users_cache: UsersCacheComponent = if custom_components.users_cache.is_some() {
            custom_components.users_cache.take().unwrap()
        } else {
            AppComponents::init_users_cache(redis, config.cache_hashing_key.clone())
        };

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
