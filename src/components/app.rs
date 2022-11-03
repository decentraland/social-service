use crate::components::{health::HealthComponent, synapse::SynapseComponent};
use crate::configuration::Config;
pub struct AppComponents {
    pub health: HealthComponent,
    pub synapse: SynapseComponent,
    pub config: Config,
}

impl AppComponents {
    pub async fn new(custom_config: Option<Config>) -> Self {
        // Initialize components
        let config =
            custom_config.unwrap_or_else(|| Config::new().expect("Couldn't read the configuratio"));

        let health = HealthComponent::default();
        let synapse = SynapseComponent::new(config.synapse.url.clone());

        // Register components to check health

        Self {
            config,
            health,
            synapse,
        }
    }
}
