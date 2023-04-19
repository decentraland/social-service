use social_service::components::database::DatabaseComponent;
use social_service::entities::friendships::FriendshipRepositoryImplementation;

use uuid::Uuid;

pub async fn add_friendship(
    db: &DatabaseComponent,
    friendship: (&str, &str),
    is_active: bool,
) -> Uuid {
    let synapse_room_id = format!("room_id_{}_{}", friendship.0, friendship.1);
    db.db_repos
        .as_ref()
        .expect("repos to be present")
        .friendships
        .create_new_friendships(friendship, is_active, &synapse_room_id, None)
        .await
        .0
        .expect("can create friendship")
}
