use crate::{
    components::{
        app::AppComponents,
        synapse::{clean_synapse_user_id, SynapseLoginRequest},
    },
    domain::error::CommonError,
};
use actix_web::ResponseError;
use actix_web::{
    get, post,
    web::{self, Data},
    HttpResponse,
};

#[get("/_matrix/client/versions")]
pub async fn version(app_data: Data<AppComponents>) -> HttpResponse {
    let version_response = app_data.synapse.get_version().await;

    match version_response {
        Ok(ok_response) => HttpResponse::Ok().json(ok_response),
        Err(err_response) => HttpResponse::from_error(err_response),
    }
}

#[post("/_matrix/client/r0/login")]
pub async fn login(
    app_data: Data<AppComponents>,
    payload: web::Json<SynapseLoginRequest>,
) -> HttpResponse {
    match app_data.synapse.login(payload.0).await {
        Ok(ok_response) => {
            let mut users_cache = app_data.users_cache.lock().await;
            let social_id = clean_synapse_user_id(&ok_response.user_id);

            if users_cache
                .add_user(
                    &ok_response.access_token,
                    &social_id,
                    &ok_response.user_id,
                    None,
                )
                .await
                .is_ok()
            {
                HttpResponse::Ok().json(ok_response)
            } else {
                log::error!(
                    "login handler: Error on storing hashed token and user id into users redis cache"
                );
                let unknown_error = CommonError::Unknown("".to_owned());
                unknown_error.error_response()
            }
        }
        Err(err_response) => err_response.error_response(),
    }
}
