use social_service::components::database::DatabaseComponent;
use social_service::entities::friendships::FriendshipRepositoryImplementation;
use uuid::Uuid;

pub async fn add_friendship(
    db: &DatabaseComponent,
    friendship: (&str, &str),
    is_active: bool,
) -> Uuid {
    let response = db
        .db_repos
        .as_ref()
        .expect("repos to be present")
        .friendships
        .create_new_friendships(friendship, is_active, None)
        .await
        .0
        .expect("can create friendship");

    response
}
