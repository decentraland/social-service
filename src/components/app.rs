use crate::components::health::HealthComponent;

#[derive(Default)]
pub struct AppComponents {
    pub health: HealthComponent,
}

impl AppComponents {
    pub async fn new() -> Self {
        // Initialize components
        let health = HealthComponent::default();

        // Register components to check health

        Self { health }
    }
}
