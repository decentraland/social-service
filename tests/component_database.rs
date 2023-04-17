mod common;

pub use common::*;

use social_service::{
    api::routes::synapse::room_events::FriendshipEvent, components::database::DBRepositories,
    entities::friendships::FriendshipRepositoryImplementation,
};
use uuid::Uuid;

#[actix_web::test]
#[serial_test::serial]
async fn should_create_and_get_a_friendship() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    create_friendship(dbrepos, "B", "A", false).await;

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

    create_friendship(dbrepos, "C", "D", false).await;

    let friendship = dbrepos
        .friendships
        .get_friendship(("C", "D"), None)
        .await
        .0
        .unwrap()
        .unwrap();

    create_friendship_event(dbrepos, friendship.id, "\"request\"", "C").await;

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

#[actix_web::test]
#[serial_test::serial]
async fn should_get_pending_request_events() {
    // create the database component
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    // create friendships between two users
    let friendship_id_1 = create_friendship(dbrepos, "A", "B", false).await;
    let friendship_id_2 = create_friendship(dbrepos, "A", "C", false).await;

    // create friendship history entries to represent friendship events
    create_friendship_event(dbrepos, friendship_id_1, "\"request\"", "A").await;
    create_friendship_event(dbrepos, friendship_id_2, "\"request\"", "A").await;
    create_friendship_event(dbrepos, friendship_id_2, "\"accept\"", "C").await;

    // retrieve the pending request events for the auth user
    let requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();

    // check that the retrieved events have the expected properties
    assert!(requests.len() == 1);
    let first_request = &requests[0];
    assert_eq!(first_request.address_1, "A");
    assert_eq!(first_request.address_2, "B");
    assert_eq!(first_request.acting_user, "A");
    assert!(first_request.metadata.is_none());
}

#[actix_web::test]
#[serial_test::serial]
async fn should_get_pending_request_events_other_acting_user() {
    // create the database component
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    // create friendships between two users
    let friendship_id_1 = create_friendship(dbrepos, "A", "B", false).await;
    let friendship_id_2 = create_friendship(dbrepos, "A", "C", false).await;

    // create friendship history entries to represent friendship events
    create_friendship_event(dbrepos, friendship_id_1, "\"request\"", "B").await;
    create_friendship_event(dbrepos, friendship_id_2, "\"request\"", "C").await;
    create_friendship_event(dbrepos, friendship_id_2, "\"accept\"", "A").await;

    // retrieve the pending request events for the auth user
    let requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();

    // check that the retrieved events have the expected properties
    assert!(requests.len() == 1);
    let first_request = &requests[0];
    assert_eq!(first_request.address_1, "A");
    assert_eq!(first_request.address_2, "B");
    assert_eq!(first_request.acting_user, "B");
    assert!(first_request.metadata.is_none());
}

/// Creates a new friendship between two users and returns the friendship_id.
async fn create_friendship(
    dbrepos: &DBRepositories,
    address_1: &str,
    address_2: &str,
    is_active: bool,
) -> Uuid {
    let synapse_room_id = format!("room_id_{}_{}", address_1, address_2);
    dbrepos
        .friendships
        .create_new_friendships((address_1, address_2), is_active, &synapse_room_id, None)
        .await
        .0
        .unwrap()
}

/// Adds a new entry to the friendship history for a given friendship_id, event type, and acting user.
async fn create_friendship_event(
    dbrepos: &DBRepositories,
    friendship_id: Uuid,
    event: &str,
    acting_user: &str,
) {
    dbrepos
        .friendship_history
        .create(friendship_id, event, acting_user, None, None)
        .await
        .0
        .unwrap();
}
