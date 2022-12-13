use std::sync::Arc;

use super::health::HealthComponent;
use super::synapse::SynapseComponent;
use super::{configuration::Config, database::DatabaseComponent, redis::RedisComponent};

use super::{
    redis::Redis,
    users_cache::{self, UsersCacheComponent},
};

pub trait IAppComponents<H: HealthComponent, S: SynapseComponent> {
    fn new(
        health_component: H,
        synapse_component: S,
        config: Config,
        db: DatabaseComponent,
        users_cache: UsersCacheComponent<Redis>,
    ) -> Self
    where
        Self: Sized;
    fn get_health_component(&self) -> &H;
    fn get_config_component(&self) -> &Config;
    fn get_synapse_component(&self) -> &S;
}

pub struct AppComponents<H: HealthComponent, S: SynapseComponent> {
    pub health: H,
    pub synapse: S,
    pub config: Config,
    pub db: DatabaseComponent,
    pub users_cache: UsersCacheComponent<Redis>,
}

impl<H: HealthComponent, S: SynapseComponent> IAppComponents<H, S> for AppComponents<H, S> {
    fn new(
        health: H,
        synapse: S,
        config: Config,
        db: DatabaseComponent,
        users_cache: UsersCacheComponent<Redis>,
    ) -> Self
    where
        Self: Sized,
    {
        Self {
            health,
            synapse,
            config,
            db,
            users_cache,
        }
    }
    fn get_health_component(&self) -> &H {
        &self.health
    }
    fn get_config_component(&self) -> &Config {
        &self.config
    }
    fn get_synapse_component(&self) -> &S {
        &self.synapse
    }
}

pub async fn new_app<
    H: HealthComponent + Default + Send + Sync + 'static,
    S: SynapseComponent + Send + Sync + 'static,
>(
    custom_config: Option<Config>,
) -> Arc<dyn IAppComponents<H, S> + Send + Sync> {
    let config =
        custom_config.unwrap_or_else(|| Config::new().expect("Couldn't read the configuration"));

    let mut health = H::default();
    let synapse = S::new(config.synapse.url.clone());

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

    Arc::new(AppComponents::new(
        health,
        synapse,
        config,
        db,
        users_cache_instance,
    ))
}
