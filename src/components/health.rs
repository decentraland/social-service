use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait Healthy {
    async fn is_healthy(&self) -> bool;
}

struct HealthComponent {
    components_to_check: Vec<Box<dyn Healthy>>,
}

impl HealthComponent {
    pub fn new() -> Self {
        let components_to_check = vec![];

        Self {
            components_to_check,
        }
    }

    pub fn register_component(&mut self, component: Box<dyn Healthy>) {
        self.components_to_check.push(component);
    }

    pub async fn calculate_status(&mut self) {
        for component in self.components_to_check.as_slice() {
            component.is_healthy().await;
        }
    }
}
