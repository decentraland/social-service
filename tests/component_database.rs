mod common;

pub use common::*;

use social_service::{
    entities::friendships::FriendshipRepositoryImplementation,
    api::routes::synapse::room_events::FriendshipEvent,
};

#[actix_web::test]
#[serial_test::serial]
async fn should_create_and_get_a_friendship() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();
    dbrepos
        .friendships
        .create_new_friendships(("B", "A"), false, None)
        .await
        .0
        .unwrap();

    let friendship = dbrepos
        .friendships
        .get_friendship(("a", "b"), None)
        .await
        .0
        .unwrap();

    assert!(friendship.is_some());

    assert_eq!(friendship.as_ref().unwrap().address_1, "A");
    assert_eq!(friendship.as_ref().unwrap().address_2, "B");
}

#[actix_web::test]
#[serial_test::serial]
async fn should_create_a_friendship_request_event() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();
    dbrepos
        .friendships
        .create_new_friendships(("C", "D"), false, None)
        .await
        .0
        .unwrap();
    let friendship = dbrepos
        .friendships
        .get_friendship(("C", "D"), None)
        .await
        .0
        .unwrap()
        .unwrap();
    dbrepos
        .friendship_history
        .create(friendship.id, "\"request\"", "C", None, None)
        .await
        .0
        .unwrap();
    let friendship_history = dbrepos
        .friendship_history
        .get_last_history_for_friendship(friendship.id, None)
        .await
        .0
        .unwrap();

    assert!(friendship_history.is_some());

    assert_eq!(
        friendship_history.as_ref().unwrap().friendship_id,
        friendship.id
    );

    assert_eq!(
        friendship_history.as_ref().unwrap().event,
        FriendshipEvent::REQUEST
    );
    assert_eq!(friendship_history.as_ref().unwrap().acting_user, "C");

    assert!(friendship_history.as_ref().unwrap().metadata.is_none());
}

#[actix_web::test]
async fn should_create_a_user_feature() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();
    dbrepos
        .user_features
        .create("A", "exposure_level", "anyone")
        .await
        .unwrap();
    let user_features = dbrepos
        .user_features
        .get_all_user_features("A")
        .await
        .unwrap();
    assert!(user_features.is_some());
    assert_eq!(user_features.as_ref().unwrap().features.len(), 1);
    assert_eq!(
        user_features
            .as_ref()
            .unwrap()
            .features
            .get(0)
            .unwrap()
            .feature_name,
        "exposure_level"
    );
    assert_eq!(
        user_features
            .as_ref()
            .unwrap()
            .features
            .get(0)
            .unwrap()
            .feature_value,
        "anyone"
    )
}
