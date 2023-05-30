mod common;

pub use common::*;
use social_service::{
    components::database::DBRepositories, friendships::RequestEventsResponse,
    ws::service::mapper::event::friendship_requests_as_request_events_response,
};

// This is a file that executes all possible flows to check the amount of pending requests
// The idea is to full test the query: 'USER_REQUESTS_QUERY' and then the mapping done by 'friendship_requests_as_request_events_response'

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 */
async fn test_1() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
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

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * B -> A accept
 */
async fn test_2() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
    match user_a_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }

    let user_b_requests = get_requests_from(dbrepos, "B").await;
    match user_b_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }
}
#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * B -> A reject
 */
async fn test_3_a() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"reject\"", "B", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
    match user_a_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }

    let user_b_requests = get_requests_from(dbrepos, "B").await;
    match user_b_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }
}
#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * B -> A reject
 * A -> B request
 */
async fn test_3_b() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"reject\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
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
#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * B -> A reject
 * B -> A request
 */
async fn test_3_c() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"reject\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "B", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
    match user_a_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert_eq!(e.incoming.clone().unwrap().total, 1);
            assert_eq!(
                e.incoming.unwrap().items[0].user.as_ref().unwrap().address,
                "B"
            );
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }

    let user_b_requests = get_requests_from(dbrepos, "B").await;
    match user_b_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert_eq!(e.outgoing.clone().unwrap().total, 1);
            assert_eq!(
                e.outgoing.unwrap().items[0].user.as_ref().unwrap().address,
                "A"
            );
        }
        _ => {
            panic!("the test failed")
        }
    }
}

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * A -> B cancel
 */
async fn test_4_a() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"cancel\"", "A", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
    match user_a_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }

    let user_b_requests = get_requests_from(dbrepos, "B").await;
    match user_b_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }
}

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * A -> B cancel
 * A -> B request
 */
async fn test_4_b() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"cancel\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
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

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * A -> B cancel
 * B -> A request
 */
async fn test_4_c() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"cancel\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "B", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
    match user_a_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert_eq!(e.incoming.clone().unwrap().total, 1);
            assert_eq!(
                e.incoming.unwrap().items[0].user.as_ref().unwrap().address,
                "B"
            );
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }

    let user_b_requests = get_requests_from(dbrepos, "B").await;
    match user_b_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert_eq!(e.outgoing.clone().unwrap().total, 1);
            assert_eq!(
                e.outgoing.unwrap().items[0].user.as_ref().unwrap().address,
                "A"
            );
        }
        _ => {
            panic!("the test failed")
        }
    }
}

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * B -> A accept
 * A -> B delete
 */
async fn test_5_a() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "A", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
    match user_a_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }

    let user_b_requests = get_requests_from(dbrepos, "B").await;
    match user_b_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }
}

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * B -> A accept
 * A -> B delete
 * A -> B request
 */
async fn test_5_b() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
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

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * B -> A accept
 * A -> B delete
 * B -> A request
 */
async fn test_5_c() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "B", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
    match user_a_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert_eq!(e.incoming.clone().unwrap().total, 1);
            assert_eq!(
                e.incoming.unwrap().items[0].user.as_ref().unwrap().address,
                "B"
            );
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }

    let user_b_requests = get_requests_from(dbrepos, "B").await;
    match user_b_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert_eq!(e.outgoing.clone().unwrap().total, 1);
            assert_eq!(
                e.outgoing.unwrap().items[0].user.as_ref().unwrap().address,
                "A"
            );
        }
        _ => {
            panic!("the test failed")
        }
    }
}

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * B -> A accept
 * B -> A delete
 */
async fn test_6_a() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "B", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
    match user_a_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }

    let user_b_requests = get_requests_from(dbrepos, "B").await;
    match user_b_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }
}

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * B -> A accept
 * B -> A delete
 * A -> B request
 */
async fn test_6_b() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
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

#[actix_web::test]
#[serial_test::serial]
/**
 * A -> B request
 * B -> A accept
 * B -> A delete
 * B -> A request
 */
async fn test_6_c() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "B", None).await;

    let user_a_requests = get_requests_from(dbrepos, "A").await;
    match user_a_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert_eq!(e.incoming.clone().unwrap().total, 1);
            assert_eq!(
                e.incoming.unwrap().items[0].user.as_ref().unwrap().address,
                "B"
            );
            assert!(e.outgoing.unwrap().items.is_empty());
        }
        _ => {
            panic!("the test failed")
        }
    }

    let user_b_requests = get_requests_from(dbrepos, "B").await;
    match user_b_requests.response.unwrap() {
        social_service::friendships::request_events_response::Response::Events(e) => {
            assert!(e.incoming.unwrap().items.is_empty());
            assert_eq!(e.outgoing.clone().unwrap().total, 1);
            assert_eq!(
                e.outgoing.unwrap().items[0].user.as_ref().unwrap().address,
                "A"
            );
        }
        _ => {
            panic!("the test failed")
        }
    }
}

async fn get_requests_from(dbrepos: &DBRepositories, user_id: &str) -> RequestEventsResponse {
    let requests = dbrepos
        .friendship_history
        .get_user_pending_request_events(user_id)
        .await
        .unwrap();

    friendship_requests_as_request_events_response(requests, user_id.to_string())
}
