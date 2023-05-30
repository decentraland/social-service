mod common;

pub use common::*;

// This is a file that executes all possible flows to check the amount of pending requests

#[actix_web::test]
#[serial_test::serial]
async fn test_1() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 0);
    assert!(user_b_requests.len() == 1);
}
#[actix_web::test]
#[serial_test::serial]
async fn test_2() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 0);
    assert!(user_b_requests.len() == 0);
}
#[actix_web::test]
#[serial_test::serial]
async fn test_3_a() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"reject\"", "B", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 0);
    assert!(user_b_requests.len() == 0);
}
#[actix_web::test]
#[serial_test::serial]
async fn test_3_b() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"reject\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 0);
    assert!(user_b_requests.len() == 1);
}
#[actix_web::test]
#[serial_test::serial]
async fn test_3_c() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"reject\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "B", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 1);
    assert!(user_b_requests.len() == 0);
}

#[actix_web::test]
#[serial_test::serial]
async fn test_4_a() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"cancel\"", "A", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 0);
    assert!(user_b_requests.len() == 0);
}

#[actix_web::test]
#[serial_test::serial]
async fn test_4_b() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"cancel\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 0);
    assert!(user_b_requests.len() == 1);
}

#[actix_web::test]
#[serial_test::serial]
async fn test_4_c() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"cancel\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "B", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 1);
    assert!(user_b_requests.len() == 0);
}

#[actix_web::test]
#[serial_test::serial]
async fn test_5_a() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "A", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 0);
    assert!(user_b_requests.len() == 0);
}

#[actix_web::test]
#[serial_test::serial]
async fn test_5_b() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 0);
    assert!(user_b_requests.len() == 1);
}

#[actix_web::test]
#[serial_test::serial]
async fn test_5_c() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "B", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 1);
    assert!(user_b_requests.len() == 0);
}

#[actix_web::test]
#[serial_test::serial]
async fn test_6_a() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "B", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 0);
    assert!(user_b_requests.len() == 0);
}

#[actix_web::test]
#[serial_test::serial]
async fn test_6_b() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 0);
    assert!(user_b_requests.len() == 1);
}

#[actix_web::test]
#[serial_test::serial]
async fn test_6_c() {
    let db = create_db_component(None).await;
    let dbrepos = db.db_repos.as_ref().unwrap();

    let a_b_friendship = create_friendship(dbrepos, "A", "B", false).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "A", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"accept\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"delete\"", "B", None).await;
    create_friendship_event(dbrepos, a_b_friendship, "\"request\"", "B", None).await;

    let user_a_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("A")
        .await
        .unwrap();
    let user_b_requests = dbrepos
        .friendship_history
        .get_user_pending_request_events("B")
        .await
        .unwrap();

    assert!(user_a_requests.len() == 1);
    assert!(user_b_requests.len() == 0);
}
