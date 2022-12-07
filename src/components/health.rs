use std::{collections::HashMap, fmt, sync::Arc};

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
    component: Arc<dyn Healthy + Send + Sync>,
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
    pub fn register_component(&mut self, component: Arc<dyn Healthy + Send + Sync>, name: String) {
        self.components_to_check
            .push(ComponentToCheck { component, name });
    }

    #[tracing::instrument(name = "Calculate components status")]
    pub async fn calculate_status(&self) -> HashMap<String, ComponentHealthStatus> {
        let mut result = HashMap::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel(self.components_to_check.len());

        for component in self.components_to_check.as_slice() {
            let tx_cloned = tx.clone();
            let component_cloned = component.component.clone();
            let component_name = component.name.clone();
            log::debug!("About to check: {}", component_name);
            tokio::spawn(async move {
                let healthy = component_cloned.is_healthy().await;
                tx_cloned.send((component_name, healthy)).await.unwrap()
            });
            log::debug!("Spawned: {}", component.name);
        }
        drop(tx);

        while let Some((name, healthy)) = rx.recv().await {
            result.insert(
                name,
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
