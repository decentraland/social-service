use crate::{
    components::{health::HealthComponent, synapse::SynapseComponent},
    AppData,
};

use actix_web::HttpResponse;

pub async fn version<H: HealthComponent, S: SynapseComponent>(
    app_data: AppData<H, S>,
) -> HttpResponse {
    let synapse_component = app_data.get_synapse_component();
    let version_response = synapse_component.get_version().await;

    match version_response {
        Ok(ok_response) => HttpResponse::Ok().json(ok_response),
        Err(err_response) => HttpResponse::InternalServerError().json(err_response),
    }
}
