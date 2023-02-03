use social_service::components::database::DatabaseComponent;
use social_service::entities::friendship_history::FriendshipMetadata;
use social_service::entities::friendships::FriendshipRepositoryImplementation;
use sqlx::types::Json;
use uuid::Uuid;

pub async fn add_friendship(db: &DatabaseComponent, friendship: (&str, &str), is_active: bool) {
    db.db_repos
        .as_ref()
        .expect("repos to be present")
        .friendships
        .create_new_friendships(friendship, is_active, None)
        .await
        .0
        .expect("can create friendship");
}

pub async fn create_friendship_history(
    db: &DatabaseComponent,
    friendship_id: Uuid,
    event: &str,
    acting_user: &str,
    metadata: Option<Json<FriendshipMetadata>>,
) {
    db.db_repos
        .as_ref()
        .expect("respos to be present")
        .friendship_history
        .create(friendship_id, event, acting_user, metadata, None)
        .await
        .0
        .expect("can create an entry for friendship_history");
}
