use actix_web::{get, web::Data, HttpResponse};

use crate::components::app::AppComponents;

#[get("/live")]
pub async fn live(_app_data: Data<AppComponents>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
