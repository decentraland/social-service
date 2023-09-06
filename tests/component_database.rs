mod common;

pub use common::*;

use social_service::{
    components::database::{DBRepositories, DatabaseComponentImplementation},
    domain::friendship_event::FriendshipEvent,
    entities::{
        friendship_history::FriendshipMetadata, friendships::FriendshipRepositoryImplementation,
    },
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
    assert_eq!(friendship.as_ref().unwrap().synapse_room_id, "room_id_B_A");
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

    let synapse_room_id = "room_id_C_D".to_string();
    let metadata = Some(sqlx::types::Json(FriendshipMetadata {
        message: None,
        synapse_room_id: Some(synapse_room_id),
        migrated_from_synapse: None,
    }));

    create_friendship_event(dbrepos, friendship.id, "\"request\"", "C", metadata).await;

    let friendship_history = dbrepos
        .friendship_history
        .get_last_history_for_friendship(friendship.id, None)
        .await
        .0
        .unwrap();

    assert!(friendship_history.is_some());
    let friendship_history = friendship_history.as_ref().unwrap();

    assert_eq!(friendship_history.friendship_id, friendship.id);
    assert_eq!(friendship_history.event, FriendshipEvent::REQUEST);
    assert_eq!(friendship_history.acting_user, "C");
    assert!(friendship_history.metadata.is_some());
    assert!(friendship_history
        .metadata
        .as_ref()
        .unwrap()
        .synapse_room_id
        .is_some());
    assert!(friendship_history
        .metadata
        .as_ref()
        .unwrap()
        .message
        .is_none());
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
    let user_features = user_features.as_ref().unwrap();

    assert_eq!(user_features.features.len(), 1);
    assert_eq!(
        user_features.features.get(0).unwrap().feature_name,
        "exposure_level"
    );
    assert_eq!(
        user_features.features.get(0).unwrap().feature_value,
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
    create_friendship_event(dbrepos, friendship_id_1, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, friendship_id_2, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, friendship_id_2, "\"accept\"", "C", None).await;

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
    create_friendship_event(dbrepos, friendship_id_1, "\"request\"", "B", None).await;
    create_friendship_event(dbrepos, friendship_id_2, "\"request\"", "C", None).await;
    create_friendship_event(dbrepos, friendship_id_2, "\"accept\"", "A", None).await;

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

#[actix_web::test]
#[serial_test::serial]
async fn should_run_transaction_succesfully() {
    // create the database component
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();
    let addresses = ("1", "2");
    let addresses_2 = ("2", "3");

    let trans = db.start_transaction().await.unwrap();

    let (_res, trans) = dbrepos
        .friendships
        .create_new_friendships(addresses, true, "room_id_1_2", Some(trans))
        .await;

    let (_res, trans) = dbrepos
        .friendships
        .create_new_friendships(addresses_2, true, "room_id_2_3", trans)
        .await;

    // Read from pre transaction status
    let (read, _) = dbrepos.friendships.get_friendship(addresses, None).await;

    match read {
        Ok(read) => {
            assert!(read.is_none())
        }
        Err(err) => panic!("Failed while reading from db {err}"),
    }

    let (read, trans) = dbrepos.friendships.get_friendship(addresses, trans).await;

    match read {
        Ok(read) => {
            assert!(read.is_some())
        }
        Err(err) => panic!("Failed while reading from db {err}"),
    }

    trans.unwrap().commit().await.unwrap();

    let (read, _) = dbrepos.friendships.get_friendship(addresses, None).await;

    match read {
        Ok(read) => {
            assert!(read.is_some())
        }
        Err(err) => panic!("Failed while reading from db {err}"),
    }
}

/// Creates a new friendship between two users and returns the friendship_id.
async fn create_friendship(
    dbrepos: &DBRepositories,
    address_1: &str,
    address_2: &str,
    is_active: bool,
) -> Uuid {
    let synapse_room_id = format!("room_id_{address_1}_{address_2}");
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
    metadata: Option<sqlx::types::Json<FriendshipMetadata>>,
) {
    dbrepos
        .friendship_history
        .create(friendship_id, event, acting_user, metadata, None)
        .await
        .0
        .unwrap();
}
