use actix_web::{
    get,
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};

use super::{errors::FriendshipsError, types::FriendshipsResponse};
use crate::{
    components::app::AppComponents, entities::friendships::FriendshipRepositoryImplementation,
    middlewares::check_auth::UserId, routes::v1::error::CommonError,
};

#[get("/v1/friendships/{userId}/mutuals")]
pub async fn get_mutual_friends(
    req: HttpRequest,
    user_id: web::Path<String>,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, FriendshipsError> {
    let logged_in_user = req
        .extensions()
        .get::<UserId>()
        .expect("to have a UserId")
        .0
        .clone();

    // Look for friendships and build friend addresses list
    match &app_data.db.db_repos {
        Some(repos) => {
            let (friendships, _) = repos
                .friendships
                .get_mutual_friends(&logged_in_user, &user_id, None)
                .await;
            match friendships {
                Err(_) => Err(FriendshipsError::CommonError(CommonError::Unknown)),
                Ok(friendships) => {
                    let response = FriendshipsResponse::new(friendships);
                    Ok(HttpResponse::Ok().json(response))
                }
            }
        }
        None => Err(FriendshipsError::CommonError(CommonError::NotFound)),
    }
}
