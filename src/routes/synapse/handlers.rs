use actix_web::{get, HttpResponse};

use crate::AppData;

#[get("/_matrix/client/versions")]
pub async fn version(app_data: AppData) -> HttpResponse {
    let version_response = app_data.get_synapse_component().get_version().await;

    match version_response {
        Ok(ok_response) => HttpResponse::Ok().json(ok_response),
        Err(err_response) => HttpResponse::InternalServerError().json(err_response),
    }
}
