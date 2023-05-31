mod common;

pub use common::*;
use social_service::{
    components::database::DatabaseComponentImplementation,
    db::{
        friendships_handler::{get_last_history, update_friendship_status},
        types::FriendshipDbRepositories,
    },
    domain::{
        friendship_event::FriendshipEvent, friendship_event_validator::validate_new_event,
        friendship_status::FriendshipStatus,
        friendship_status_calculator::get_new_friendship_status, room::RoomInfo,
    },
};

// This is a file that executes all possible flows to emulate the event updates
// Makes the call to the db functions: 'get_last_history', 'validate_new_event' & 'get_new_friendship_status'
// Then executes the flow of getting pending requests

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 */
async fn test_1() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let acting_user = "A";
    let second_user = "B";
    let friendship = create_friendship(dbrepos, acting_user, second_user, false).await;
    let room_event = FriendshipEvent::REQUEST;

    let last_history = get_last_history(&dbrepos.friendship_history, &friendship)
        .await
        .unwrap();
    assert!(last_history.is_none());

    let new_event_validation = validate_new_event(&last_history, room_event);
    assert!(new_event_validation.is_ok());

    let new_status = get_new_friendship_status(acting_user, &last_history, room_event).unwrap();
    assert_eq!(
        new_status,
        FriendshipStatus::Requested(acting_user.to_string())
    );

    let room_info = RoomInfo {
        room_event,
        room_message_body: None,
        room_id: "A_B",
    };
    let friendship_ports = FriendshipDbRepositories {
        db: &db,
        friendships_repository: &dbrepos.friendships,
        friendship_history_repository: &dbrepos.friendship_history,
    };
    let transaction = db.start_transaction().await.unwrap();

    let result = update_friendship_status(
        &friendship,
        acting_user,
        second_user,
        new_status,
        room_info,
        friendship_ports,
        transaction,
    )
    .await;
    assert!(result.is_ok());

    let user_a_requests = get_requests_from(dbrepos, acting_user).await;
    match user_a_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert_eq!(e.outgoing.clone().unwrap().total, 1);
            assert_eq!(
                e.outgoing.unwrap().items[0].user.as_ref().unwrap().address,
                "B"
            );
        }
        _ => {
            panic!("the test failed")
        }
    }

    let user_b_requests = get_requests_from(dbrepos, "B").await;
    match user_b_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert_eq!(e.incoming.clone().unwrap().total, 1);
            assert_eq!(
                e.incoming.unwrap().items[0].user.as_ref().unwrap().address,
                "A"
            );
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }
}
