use clap::App;

use crate::components::health::HealthComponent;

#[derive(Default)]
pub struct AppComponents {
    pub health_component: HealthComponent,
}

impl AppComponents {
    pub async fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}
