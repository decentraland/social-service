use crate::components::health::HealthComponent;

#[derive(Default)]
pub struct AppComponents {
    pub health_component: HealthComponent,
}

impl AppComponents {
    pub async fn new() -> Self {
        Self {
            ..Default::default() 
            // initialize components

            // register for health checking
        }
    }
}
