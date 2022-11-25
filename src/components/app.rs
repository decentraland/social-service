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
        env_logger::init();
        // Initialize components
        let config =
            custom_config.unwrap_or_else(|| Config::new().expect("Couldn't read the configuratio"));

        let health = HealthComponent::default();
        let synapse = SynapseComponent::new(config.synapse.url.clone());
        let mut db = DatabaseComponent::new(&config.db);
        match db.run().await {
            Err(err) => {
                log::debug!("Error on running the DB: {:?}", err);
                panic!("Unable to run the DB")
            }
            _ => {}
        }

        Self {
            config,
            health,
            synapse,
            db,
        }
    }
}
