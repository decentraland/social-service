use sea_orm::DatabaseConnection;
use std::sync::Arc;

const TABLE: &str = "user_features";

#[derive(Clone)]
pub struct UserFeaturesRepository {
    db_connection: Arc<Option<DatabaseConnection>>,
}

impl UserFeaturesRepository {
    pub fn new(db: Arc<Option<DatabaseConnection>>) -> Self {
        Self { db_connection: db }
    }
}
