use actix_web::{
    get,
    web::{self, Data},
    HttpResponse,
};

use super::types::FriendshipsResponse;
use crate::components::app::AppComponents;

#[get("/v1/friendships/{userId}")]
pub async fn get_user_friends(
    user_id: web::Path<String>,
    _app_data: Data<AppComponents>,
) -> HttpResponse {
    let permissions = true;

    if !permissions {
        return HttpResponse::Forbidden().json(format!(
            "You don't have permission to view {} friends",
            user_id
        ));
    }

    let addresses = vec!["0xa1", "0xa2"]; // get addresses from the database

    let response: FriendshipsResponse = FriendshipsResponse::new(addresses);

    return HttpResponse::Ok().json(response);
}
