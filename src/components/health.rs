use std::{collections::HashMap, fmt};

use async_trait::async_trait;

use crate::routes::health::health::ComponentHealthStatus;

#[async_trait]
pub trait Healthy {
    async fn is_healthy(&self) -> bool;
}

struct ComponentToCheck {
    component: Box<dyn Healthy + Send + Sync>,
    name: String,
}

impl std::fmt::Debug for ComponentToCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentToCheck")
            .field("name", &self.name)
            .finish()
    }
}

#[derive(Default, Debug)]
pub struct HealthComponent {
    components_to_check: Vec<ComponentToCheck>,
}

impl HealthComponent {
    pub fn register_component(&mut self, component: Box<dyn Healthy + Send + Sync>, name: String) {
        self.components_to_check
            .push(ComponentToCheck { component, name });
    }

    #[tracing::instrument(name = "Calculate components status")]
    pub async fn calculate_status(&self) -> HashMap<String, ComponentHealthStatus> {
        let mut result = HashMap::new();

        // TODO: Parallelize this checks
        for component in self.components_to_check.as_slice() {
            let healthy = component.component.is_healthy().await;
            result.insert(
                component.name.to_string(),
                ComponentHealthStatus {
                    component: component.name.to_string(),
                    healthy,
                },
            );
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::components::numbers::Numbers;

    use super::HealthComponent;

    #[actix_web::test]
    async fn health_check_checks_all_components() {
        let component1 = Box::new(Numbers::default());
        let component2 = Box::new(Numbers::default());

        let component1_name = "numbers".to_string();
        let component2_name = "numbers2".to_string();

        let mut health_component = HealthComponent::default();

        health_component.register_component(component1, component1_name.to_string());
        health_component.register_component(component2, component2_name.to_string());

        let res = health_component.calculate_status().await;

        let res_1 = res.get(&component1_name);
        let res_2 = res.get(&component2_name);

        assert!(res_1.is_some());
        assert!(res_2.is_some());
    }
}
