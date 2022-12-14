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
        return Err(FriendshipsError::CommonError(CommonError::Forbidden(
            format!("You don't have permission to view {} friends", user_id),
        )));
    }

    let user = user_id.as_str();

    let res = app_data
        .as_ref()
        .db
        .db_repos
        .as_ref()
        .unwrap()
        .friendships
        .get_user_friends(user, false)
        .await;

    if res.is_err() {
        return Err(FriendshipsError::CommonError(CommonError::Unknown));
    }

    let friendships = res.unwrap();

    let addresses = friendships
        .iter()
        .map(|friendship| -> &str {
            if friendship.address_1.eq_ignore_ascii_case(user) {
                return friendship.address_2.as_str();
            }
            friendship.address_1.as_str()
        })
        .collect::<Vec<&str>>();

    let response: FriendshipsResponse = FriendshipsResponse::new(addresses);

    return Ok(HttpResponse::Ok().json(response));
}
