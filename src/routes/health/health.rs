use std::collections::HashMap;

use actix_web::{get, web::Data, HttpResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::components::app::AppComponents;

#[derive(Debug, Default, Serialize)]
struct HealthStatus {
    version: String,
    checks: HashMap<String, ComponentHealthStatus>,
}

#[derive(Debug, Default, Serialize)]
struct ComponentHealthStatus {
    component: String,
    component_type: String,
    healthy: bool,
}

#[get("/health")]
pub async fn health(app_data: Data<AppComponents>) -> HttpResponse {
    let mut result = HealthStatus::default();

    result.version = "0.0.1".to_string();

    HttpResponse::Ok().json(result)
}
