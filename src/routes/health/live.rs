use actix_web::{get, web::Data, HttpResponse};

use crate::components::app::AppComponents;

#[get("/health/live")]
pub async fn live(_app_data: Data<AppComponents>) -> HttpResponse {
    HttpResponse::Ok().json("alive")
}
