use crate::components::health::HealthComponent;

use super::numbers::Numbers;

#[derive(Default)]
pub struct AppComponents {
    pub health: HealthComponent,
    pub numbers: Numbers,
}

impl AppComponents {
    pub async fn new() -> Self {
        // Initialize components
        let mut health = HealthComponent::default();
        let numbers = Numbers::default();

        // Register components to check health
        health.register_component(Box::new(numbers.clone()), "numbers".to_string());

        Self {
            numbers,
            health,
            ..Default::default()
        }
    }
}
