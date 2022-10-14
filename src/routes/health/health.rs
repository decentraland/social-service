use std::collections::HashMap;

use actix_web::{get, web::Data, HttpResponse};

use serde::Serialize;

use crate::components::app::AppComponents;

#[derive(Debug, Default, Serialize)]
struct HealthStatus {
    version: String,
    checks: HashMap<String, ComponentHealthStatus>,
    healthy: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct ComponentHealthStatus {
    pub component: String,
    pub healthy: bool,
}

#[get("/health")]
pub async fn health(app_data: Data<AppComponents>) -> HttpResponse {
    let mut result = HealthStatus::default();

    result.version = "0.0.1".to_string();
    result.checks = app_data.health.calculate_status().await;

    HttpResponse::Ok().json(result)
}
