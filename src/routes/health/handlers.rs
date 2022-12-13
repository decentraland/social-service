use std::collections::HashMap;

use actix_web::{http::StatusCode, HttpResponse};

use serde::Serialize;

use super::consts::{FAIL, FAILED_STATUS, SUCCESSFUL_STATUS};
use crate::{
    components::{health::HealthComponent, synapse::SynapseComponent},
    routes::health::consts::MIME,
    AppData,
};

#[derive(Debug, Default, Serialize)]
struct HealthStatus {
    checks: HashMap<String, ComponentHealthStatus>,
}

#[derive(Debug, Default, Serialize)]
pub struct ComponentHealthStatus {
    pub status: String,
}

#[derive(Debug, Default, Serialize)]
struct ReadinessResponse {
    details: HashMap<String, ComponentHealthStatus>,
    status: String,
}

pub async fn is_app_healthy<H: HealthComponent, S: SynapseComponent>(
    app_data: AppData<H, S>,
) -> HttpResponse {
    let mut result = HealthStatus::default();

    let health_component = app_data.get_health_component();

    result.checks = health_component.calculate_status().await;
    let is_ready = !result
        .checks
        .values()
        .any(|value| value.status.eq_ignore_ascii_case(FAIL));

    let status = if is_ready {
        SUCCESSFUL_STATUS
    } else {
        FAILED_STATUS
    };

    let response: ReadinessResponse = ReadinessResponse {
        details: result.checks,
        status: if is_ready {
            SUCCESSFUL_STATUS.to_string()
        } else {
            FAILED_STATUS.to_string()
        },
    };

    HttpResponse::Ok()
        .status(StatusCode::from_u16(status).unwrap())
        .content_type(MIME)
        .json(response)
}

/**
 * Readiness probes indicate whether your application is ready to
 * handle requests. It could be that your application is alive, but
 * that it just can't handle HTTP traffic. In that case, Kubernetes
 * won't kill the container, but it will stop sending it requests.
 * In practical terms, that means the pod is removed from an
 * associated service's "pool" of pods that are handling requests,
 * by marking the pod as "Unready".
 */

pub async fn health<H: HealthComponent, S: SynapseComponent>(
    app_data: AppData<H, S>,
) -> HttpResponse {
    is_app_healthy(app_data).await
}

/**
 * The first probe to run is the Startup probe.
 * When your app starts up, it might need to do a lot of work.
 * It might need to fetch data from remote services, load dlls
 * from plugins, who knows what else. During that process, your
 * app should either not respond to requests, or if it does, it
 * should return a status code of 400 or higher. Once the startup
 * process has finished, you can switch to returning a success
 * res (200) for the startup probe.
 */
pub async fn startup<H: HealthComponent, S: SynapseComponent>(
    app_data: AppData<H, S>,
) -> HttpResponse {
    is_app_healthy(app_data).await
}

pub async fn live() -> HttpResponse {
    HttpResponse::Ok().json("alive")
}
