use sea_orm::DatabaseConnection;
use std::sync::Arc;

const TABLE: &str = "friendship_history";

#[derive(Clone)]
pub struct FriendshipHistoryRepository {
    db_connection: Arc<Option<DatabaseConnection>>,
}

impl FriendshipHistoryRepository {
    pub fn new(db: Arc<Option<DatabaseConnection>>) -> Self {
        Self { db_connection: db }
    }
}
