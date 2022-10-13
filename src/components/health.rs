use std::collections::HashMap;

use async_trait::async_trait;

#[async_trait]
pub trait Healthy {
    async fn is_healthy(&self) -> bool;
}

struct ComponentToCheck {
    component: Box<dyn Healthy + Send + Sync>,
    name: String,
}

#[derive(Default)]
pub struct HealthComponent {
    components_to_check: Vec<ComponentToCheck>,
}

impl HealthComponent {
    pub fn register_component(&mut self, component: Box<dyn Healthy + Send + Sync>, name: String) {
        self.components_to_check
            .push(ComponentToCheck { component, name });
    }

    pub async fn calculate_status(&self) -> HashMap<String, bool> {
        let mut result = HashMap::new();
        for component in self.components_to_check.as_slice() {
            let is_healthy = component.component.is_healthy().await;
            result.insert(component.name.to_string(), is_healthy);
        }

        result
    }
}
