use std::{collections::HashMap, sync::Arc};

use sqlx::{
    postgres::Postgres,
    query::Query,
    types::{Json, Uuid},
    Error, Row,
};

use crate::{
    components::database::{DBConnection, DatabaseComponent},
    generate_uuid_v4,
};

#[derive(Clone)]
pub struct FriendshipHistoryRepository {
    db_connection: Arc<Option<DBConnection>>,
}

pub struct FriendshipHistory {
    pub friendship_id: Uuid,
    pub event: String,
    pub acting_user: String,
    pub metadata: Option<Json<HashMap<String, String>>>,
}

impl FriendshipHistoryRepository {
    pub fn new(db: Arc<Option<DBConnection>>) -> Self {
        Self { db_connection: db }
    }

    pub fn create_query<'a>(
        &self,
        friendship_id: Uuid,
        event: &'a str,
        acting_user: &'a str,
        metadata: Option<Json<HashMap<String, String>>>,
    ) -> Query<'a, Postgres, sqlx::postgres::PgArguments> {
        sqlx::query(
                "INSERT INTO friendship_history (id,friendship_id, event, acting_user, metadata) VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(Uuid::parse_str(generate_uuid_v4().as_str()).unwrap())
        .bind(friendship_id)
        .bind(event)
        .bind(acting_user)
        .bind(metadata)
    }

    pub async fn create(
        &self,
        friendship_id: Uuid,
        event: &str,
        acting_user: &str,
        metadata: Option<Json<HashMap<String, String>>>,
    ) -> Result<(), sqlx::Error> {
        let db_conn = DatabaseComponent::get_connection(&self.db_connection);
        match self
            .create_query(friendship_id, event, acting_user, metadata)
            .execute(db_conn)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn get(&self, friendship_id: Uuid) -> Result<Option<FriendshipHistory>, sqlx::Error> {
        let db_conn = DatabaseComponent::get_connection(&self.db_connection);
        match sqlx::query("SELECT * FROM friendship_history where friendship_id = $1")
            .bind(friendship_id)
            .fetch_one(db_conn)
            .await
        {
            Ok(row) => {
                let history = FriendshipHistory {
                    friendship_id: row.try_get("friendship_id").unwrap(),
                    event: row.try_get("event").unwrap(),
                    acting_user: row.try_get("acting_user").unwrap(),
                    metadata: row.try_get("metadata").unwrap(),
                };
                Ok(Some(history))
            }
            Err(err) => match err {
                Error::RowNotFound => Ok(None),
                _ => Err(err),
            },
        }
    }
}
