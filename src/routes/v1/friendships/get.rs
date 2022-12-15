use actix_web::{
    get,
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};

use super::{errors::FriendshipsError, types::FriendshipsResponse};
use crate::{
    components::app::AppComponents, middlewares::check_auth::UserId, routes::v1::error::CommonError,
};

#[get("/v1/friendships/{userId}")]
pub async fn get_user_friends(
    req: HttpRequest,
    user_id: web::Path<String>,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, FriendshipsError> {
    get_user_friends_handler(req, user_id, app_data).await
}

async fn get_user_friends_handler(
    req: HttpRequest,
    user_id: web::Path<String>,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, FriendshipsError> {
    let extensions = req.extensions_mut();
    let logged_in_user = extensions.get::<UserId>().unwrap();

    // for the moment allow only for users to query their own friends
    let permissions = user_id
        .as_str()
        .eq_ignore_ascii_case(logged_in_user.0.as_str());

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

#[cfg(test)]
mod tests {
    use actix_web::{test, HttpRequest};

    use super::get_user_friends_handler;

    // use super::get_user_friends;

    #[actix_web::test]
    async fn test_get_user_friends() {
        let other_user_id = "test";

        let req = test::TestRequest::default()
            .uri(format!("/v1/friendships/{other_user_id}").as_str())
            .to_http_request();

        // let response = get_user_friends_handler(req, req.path(), "asd").await;
    }
}
