use std::{collections::HashMap, fmt};

use async_trait::async_trait;

use crate::routes::health::{
    consts::{FAIL, PASS},
    handlers::ComponentHealthStatus,
};

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

#[async_trait::async_trait]
pub trait HealthComponent {
    fn register_component(&mut self, component: Box<dyn Healthy + Send + Sync>, name: String);
    async fn calculate_status(&self) -> HashMap<String, ComponentHealthStatus>;
}

#[derive(Default, Debug)]
pub struct Health {
    components_to_check: Vec<ComponentToCheck>,
}

#[async_trait::async_trait]
impl HealthComponent for Health {
    fn register_component(&mut self, component: Box<dyn Healthy + Send + Sync>, name: String) {
        self.components_to_check
            .push(ComponentToCheck { component, name });
    }

    #[tracing::instrument(name = "Calculate components status")]
    async fn calculate_status(&self) -> HashMap<String, ComponentHealthStatus> {
        let mut result = HashMap::new();

        // TODO: Parallelize this checks
        for component in self.components_to_check.as_slice() {
            let healthy = component.component.is_healthy().await;
            result.insert(
                component.name.to_string(),
                ComponentHealthStatus {
                    status: if healthy {
                        PASS.to_string()
                    } else {
                        FAIL.to_string()
                    },
                },
            );
        }

        result
    }
}
