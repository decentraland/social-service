use actix_web::{
    get,
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};

use super::{errors::FriendshipsError, types::FriendshipsResponse};
use crate::{
    components::{app::AppComponents, database::DatabaseComponent},
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
        let extensions = req.extensions_mut();
        let user_id = extensions.get::<UserId>();
        let res = user_id.unwrap();
        res.0.clone()
    };

    let response = get_user_friends_handler(logged_in_user.as_str(), user_id.as_str(), &app_data.db).await;

    match response {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(err) => Err(err),
    }
}

async fn get_user_friends_handler(
    logged_in_user: &str,
    user_id: &str,
    db: &DatabaseComponent,
) -> Result<FriendshipsResponse, FriendshipsError> {
    // for the moment allow only for users to query their own friends
    let permissions = user_id.eq_ignore_ascii_case(logged_in_user);

    if !permissions {
        return Err(FriendshipsError::CommonError(CommonError::Forbidden(
            format!("You don't have permission to view {user_id} friends"),
        )));
    }

    let res = db
        .get_repos()
        .as_ref()
        .unwrap()
        .get_friendships()
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

    Ok(FriendshipsResponse::new(addresses))
}

#[cfg(test)]
mod tests {

    use uuid::Uuid;

    use crate::{
        components::database::{DBRepositories, DatabaseComponent},
        entities::friendships::{Friendship, FriendshipsRepository},
        generate_uuid_v4,
        routes::v1::{
            error::CommonError,
            friendships::{errors::FriendshipsError, types::FriendshipFriend},
        },
    };

    use super::get_user_friends_handler;

    #[actix_web::test]
    async fn test_get_user_friends() {
        let mut mocked_db = DatabaseComponent::faux();

        unsafe {
            faux::when!(mocked_db.get_repos).then_unchecked_return(&None);
        }

        let response = get_user_friends_handler("user1", "user2", &mocked_db).await;

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
        let mut mocked_db = DatabaseComponent::faux();
        let mut mocked_repos = DBRepositories::faux();
        let mut mocked_friendship = FriendshipsRepository::faux();

        unsafe {
            faux::when!(mocked_friendship.get_user_friends)
                .then_unchecked(|_| Err(sqlx::Error::RowNotFound));
            faux::when!(mocked_repos.get_friendships).then_unchecked_return(&mocked_friendship);
            faux::when!(mocked_db.get_repos).then_unchecked_return(&Some(mocked_repos.clone()));
        }

        let response = get_user_friends_handler("user1", "user1", &mocked_db).await;

        assert!(response.is_err());

        if let Err(res) = response {
            assert_eq!(res, FriendshipsError::CommonError(CommonError::Unknown))
        }
    }

    #[actix_web::test]
    async fn test_get_user_friends_should_return_the_address_list() {
        let user_id = "custom id";
        let other_user = "another id";
        let other_user_2 = "another id 2";

        let mut mocked_db = DatabaseComponent::faux();
        let mut mocked_repos = DBRepositories::faux();
        let mut mocked_friendship = FriendshipsRepository::faux();

        unsafe {
            faux::when!(mocked_friendship.get_user_friends).then_unchecked(|_| {
                Ok(vec![
                    Friendship {
                        id: Uuid::parse_str(generate_uuid_v4().as_str()).unwrap(),
                        address_1: user_id.to_string(),
                        address_2: other_user.to_string(),
                        is_active: true,
                    },
                    Friendship {
                        id: Uuid::parse_str(generate_uuid_v4().as_str()).unwrap(),
                        address_1: other_user_2.to_string(),
                        address_2: user_id.to_string(),
                        is_active: true,
                    },
                ])
            });

            faux::when!(mocked_repos.get_friendships).then_unchecked_return(&mocked_friendship);
            faux::when!(mocked_db.get_repos).then_unchecked_return(&Some(mocked_repos.clone()));
        }

        let response = get_user_friends_handler(user_id, user_id, &mocked_db).await;

        assert!(response.is_ok());

        let friendship_response = response.unwrap();

        assert_eq!(friendship_response.friendships.len(), 2);
        assert!(friendship_response.friendships.contains(&FriendshipFriend {
            address: other_user.to_string()
        }));
        assert!(friendship_response.friendships.contains(&FriendshipFriend {
            address: other_user_2.to_string()
        }));
    }
}
