use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::health::Healthy;

#[derive(Default, Clone)]
pub struct Numbers {
    pub items: Arc<Mutex<Vec<i64>>>,
}

impl Numbers {
    pub fn get(&self) -> Vec<i64> {
        self.items.lock().unwrap().clone()
    }
}

#[async_trait]
impl Healthy for Numbers {
    async fn is_healthy(&self) -> bool {
        return true;
    }
}
