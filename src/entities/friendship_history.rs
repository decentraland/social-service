use sea_orm::{
    prelude::{Json, Uuid},
    ConnectionTrait, DatabaseConnection, DbErr,
};

use std::sync::Arc;

use crate::components::database::DatabaseComponent;

const TABLE: &str = "friendship_history";

#[derive(Clone)]
pub struct FriendshipHistoryRepository {
    db_connection: Arc<Option<DatabaseConnection>>,
}

pub struct FriendshipHistory {
    pub friendship_id: Uuid,
    pub event: String,
    pub acting_user: String,
    pub metadata: Option<Json>,
}

impl FriendshipHistoryRepository {
    pub fn new(db: Arc<Option<DatabaseConnection>>) -> Self {
        Self { db_connection: db }
    }

    pub async fn create(
        &self,
        friendship_id: Uuid,
        event: &str,
        acting_user: &str,
        metadata: Option<Json>,
    ) -> Result<(), DbErr> {
        let query = DatabaseComponent::get_statement(
            format!("INSERT INTO {} (friendship_id, event, acting_user, metadata) VALUES ($1, $2, $3, $4)", TABLE).as_str(),
            vec![friendship_id.into(), event.into(), acting_user.into(), metadata.into()],
        );
        match self
            .db_connection
            .as_ref()
            .as_ref()
            .unwrap()
            .execute(query)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn get(&self, friendship_id: Uuid) -> Result<Option<FriendshipHistory>, DbErr> {
        let query = DatabaseComponent::get_statement(
            format!("SELECT * FROM {} WHERE friendship_id = $1", TABLE).as_str(),
            vec![friendship_id.into()],
        );
        match self
            .db_connection
            .as_ref()
            .as_ref()
            .unwrap()
            .query_one(query)
            .await
        {
            Ok(row) => {
                if row.is_none() {
                    Ok(None)
                } else {
                    let row = row.unwrap();
                    let history = FriendshipHistory {
                        friendship_id: row.try_get("", "friendship_id").unwrap(),
                        event: row.try_get("", "event").unwrap(),
                        acting_user: row.try_get("", "acting_user").unwrap(),
                        metadata: row.try_get("", "metadata").unwrap(),
                    };
                    Ok(Some(history))
                }
            }
            Err(err) => Err(err),
        }
    }
}
