use sea_orm::DatabaseConnection;
use std::sync::Arc;

const TABLE: &str = "friendship_history_events";

#[derive(Clone)]
pub struct FriendshipHistoryEventsRepository {
    db_connection: Arc<Option<DatabaseConnection>>,
}

impl FriendshipHistoryEventsRepository {
    pub fn new(db: Arc<Option<DatabaseConnection>>) -> Self {
        Self { db_connection: db }
    }
}
