use actix_web::{
    get,
    web::{self, Data},
    HttpResponse,
};

use super::{errors::FriendshipsError, types::FriendshipsResponse};
use crate::{components::app::AppComponents, routes::v1::error::CommonError};

#[get("/v1/friendships/{userId}")]
pub async fn get_user_friends(
    user_id: web::Path<String>,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, FriendshipsError> {
    let permissions = true;

    if !permissions {
        return Ok(HttpResponse::Forbidden().json(format!(
            "You don't have permission to view {} friends",
            user_id
        )));
    }

    let res = app_data
        .as_ref()
        .db
        .db_repos
        .as_ref()
        .unwrap()
        .friendships
        .get_user_friends(user_id.as_str(), false)
        .await;

    if res.is_err() {
        return Err(FriendshipsError::CommonError(CommonError::Unknown));
    }

    let response: FriendshipsResponse = FriendshipsResponse::new(vec!["a", "b"]);

    return Ok(HttpResponse::Ok().json(response));
}
