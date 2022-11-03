use crate::components::app::AppComponents;

use actix_web::{get, web::Data, HttpResponse};

#[get("/_matrix/client/versions")]
pub async fn version(app_data: Data<AppComponents>) -> HttpResponse {
    let version_response = app_data.synapse.get_version().await;

    match version_response {
        Ok(ok_response) => HttpResponse::Ok().json(ok_response),
        Err(err_response) => HttpResponse::InternalServerError().json(err_response),
    }
}
