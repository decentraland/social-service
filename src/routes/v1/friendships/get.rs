use actix_web::{
    get,
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};

use super::{errors::FriendshipsError, types::FriendshipsResponse};
use crate::{
    components::{app::AppComponents, synapse::clean_synapse_user_id},
    entities::friendships::{Friendship, FriendshipRepositoryImplementation},
    middlewares::check_auth::UserId,
    routes::v1::error::CommonError,
};

#[get("/v1/friendships/{userId}")]
pub async fn get_user_friends(
    req: HttpRequest,
    user_id: web::Path<String>,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, FriendshipsError> {
    let logged_in_user = {
        let extensions = req.extensions();
        extensions
            .get::<UserId>()
            .expect("to have a UserId")
            .0
            .clone()
    };

    // The user_id from parameter could be in matrix format
    let clean_logged_in_user = clean_synapse_user_id(logged_in_user.as_str());

    // Return error when user has no permission
    if !has_permission(clean_logged_in_user.as_str(), user_id.as_str()) {
        return Err(FriendshipsError::CommonError(CommonError::Forbidden(
            format!("You don't have permission to view {user_id} friends"),
        )));
    }

    // Look for friendships and build friend addresses list
    match &app_data.db.db_repos {
        Some(repos) => {
            let (friendships, _) = repos
                .friendships
                .get_user_friends(&user_id, true, None)
                .await;
            match friendships {
                Err(_) => Err(FriendshipsError::CommonError(CommonError::Unknown)),
                Ok(friendships) => {
                    let response = FriendshipsResponse::new(get_friends(&user_id, friendships));
                    Ok(HttpResponse::Ok().json(response))
                }
            }
        }
        None => Err(FriendshipsError::CommonError(CommonError::NotFound)),
    }
}

fn has_permission(logged_user_id: &str, user_id: &str) -> bool {
    user_id.eq_ignore_ascii_case(logged_user_id)
}

fn get_friends(user_id: &str, friendships: Vec<Friendship>) -> Vec<String> {
    friendships
        .iter()
        .map(
            |friendship| match friendship.address_1.eq_ignore_ascii_case(user_id) {
                true => friendship.address_2.to_string(),
                false => friendship.address_1.to_string(),
            },
        )
        .collect()
}
