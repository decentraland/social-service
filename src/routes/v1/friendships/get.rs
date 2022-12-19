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
    let extensions = req.extensions_mut();
    let logged_in_user = extensions.get::<UserId>().unwrap();

    get_user_friends_handler(logged_in_user.0.as_str(), user_id.as_str(), app_data).await
}

async fn get_user_friends_handler(
    logged_in_user: &str,
    user_id: &str,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, FriendshipsError> {
    // for the moment allow only for users to query their own friends
    let permissions = user_id.eq_ignore_ascii_case(logged_in_user);

    if !permissions {
        return Err(FriendshipsError::CommonError(CommonError::Forbidden(
            format!("You don't have permission to view {} friends", user_id),
        )));
    }

    let res = app_data
        .as_ref()
        .db
        .get_repos()
        .unwrap()
        .friendships
        .get_user_friends(user_id, false)
        .await;

    if res.is_err() {
        return Err(FriendshipsError::CommonError(CommonError::Unknown));
    }

    let friendships = res.unwrap();

    let addresses = friendships
        .iter()
        .map(|friendship| -> &str {
            if friendship.address_1.eq_ignore_ascii_case(user_id) {
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
    use actix_web::{test, web::Data};

    use crate::{
        components::{
            app::{AppComponents, CustomComponents},
            configuration::Config,
            database::DatabaseComponent,
            redis::Redis,
            synapse::SynapseComponent,
            users_cache::UsersCacheComponent,
        },
        routes::v1::{error::CommonError, friendships::errors::FriendshipsError},
    };

    use super::get_user_friends_handler;

    // use super::get_user_friends;

    #[actix_web::test]
    async fn test_get_user_friends() {
        let cfg = Config::new().unwrap();

        let mocked_synapse = SynapseComponent::faux();
        let mut mocked_db = DatabaseComponent::faux();
        let mocked_users_cache = UsersCacheComponent::faux();
        let mocked_redis = Redis::faux();

        faux::when!(mocked_db.get_repos).then(|_| None);

        let mocked_components = CustomComponents {
            synapse: Some(mocked_synapse),
            db: Some(mocked_db),
            users_cache: Some(mocked_users_cache),
            redis: Some(mocked_redis),
        };

        let app_data = Data::new(AppComponents::new(Some(cfg), Some(mocked_components)).await);

        let response = get_user_friends_handler("user1", "user2", app_data).await;

        assert!(response.is_err());

        if let Err(res) = response {
            assert_eq!(
                res,
                FriendshipsError::CommonError(CommonError::Forbidden("".to_string()))
            )
        }
    }

    #[actix_web::test]
    async fn test_get_user_friends_database_error_should_return_unknown_error() {
        let cfg = Config::new().unwrap();

        let mocked_synapse = SynapseComponent::faux();
        let mut mocked_db = DatabaseComponent::faux();
        let mocked_users_cache = UsersCacheComponent::faux();
        let mocked_redis = Redis::faux();

        faux::when!(mocked_db.get_repos).then(|_| None);

        let mocked_components = CustomComponents {
            synapse: Some(mocked_synapse),
            db: Some(mocked_db),
            users_cache: Some(mocked_users_cache),
            redis: Some(mocked_redis),
        };

        let app_data = Data::new(AppComponents::new(Some(cfg), Some(mocked_components)).await);

        let response = get_user_friends_handler("user1", "user1", app_data).await;

        assert!(response.is_err());

        if let Err(res) = response {
            assert_eq!(res, FriendshipsError::CommonError(CommonError::Unknown))
        }
    }
}
