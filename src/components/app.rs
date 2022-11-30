use crate::components::{
    configuration::Config, database::DatabaseComponent, health::HealthComponent,
    synapse::SynapseComponent,
};

pub struct AppComponents {
    pub health: HealthComponent,
    pub synapse: SynapseComponent,
    pub config: Config,
    pub db: DatabaseComponent,
}

impl AppComponents {
    pub async fn new(custom_config: Option<Config>) -> Self {
        if let Err(_) = env_logger::try_init() {
            log::debug!("Logger already init")
        }

        // Initialize components
        let config =
            custom_config.unwrap_or_else(|| Config::new().expect("Couldn't read the configuratio"));

        let mut health = HealthComponent::default();
        let synapse = SynapseComponent::new(config.synapse.url.clone());
        let mut db = DatabaseComponent::new(&config.db);

        if let Err(err) = db.run().await {
            log::debug!("Error on running the DB: {:?}", err);
            panic!("Unable to run the DB")
        }

        health.register_component(Box::new(db.clone()), "database".to_string());

        Self {
            config,
            health,
            synapse,
            db,
        }
    }
}
