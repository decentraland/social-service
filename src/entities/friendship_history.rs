use std::{collections::HashMap, sync::Arc};

use chrono::NaiveDateTime;
use mockall::predicate::*;
use sqlx::{
    postgres::Postgres,
    query::Query,
    types::{Json, Uuid},
    Error, Row, Transaction,
};

use crate::{
    components::database::{DBConnection, DatabaseComponent, Executor},
    generate_uuid_v4,
    routes::synapse::room_events::FriendshipEvent,
};

use super::utils::get_transaction_result_from_executor;

#[derive(Clone)]
pub struct FriendshipHistoryRepository {
    db_connection: Arc<Option<DBConnection>>,
}

pub struct FriendshipHistory {
    pub friendship_id: Uuid,
    pub event: FriendshipEvent,
    pub acting_user: String,
    pub timestamp: NaiveDateTime,
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

    pub async fn create<'a>(
        &'a self,
        friendship_id: Uuid,
        event: &'a str,
        acting_user: &'a str,
        metadata: Option<Json<HashMap<String, String>>>,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (Result<(), sqlx::Error>, Option<Transaction<'a, Postgres>>) {
        let executor = self.get_executor(transaction);

        let query = self.create_query(friendship_id, event, acting_user, metadata);

        let (res, resulting_executor) = DatabaseComponent::execute_query(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match res {
            Ok(_) => (Ok(()), transaction_to_return),
            Err(err) => {
                log::error!("Error while creating friendship history {err}");
                (Err(err), transaction_to_return)
            }
        }
    }

    pub async fn get_last_history_for_friendship<'a>(
        &'a self,
        friendship_id: Uuid,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Option<FriendshipHistory>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    ) {
        let executor = self.get_executor(transaction);
        let query = sqlx::query("SELECT * FROM friendship_history where friendship_id = $1 ORDER BY timestamp DESC LIMIT 1")
            .bind(friendship_id);

        let (res, resulting_executor) = DatabaseComponent::fetch_one(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match res {
            Ok(row) => {
                let friendship_id = row.try_get("friendship_id").unwrap();
                let event = serde_json::from_str::<FriendshipEvent>(row.try_get("event").unwrap());

                if event.is_err() {
                    let err = event.unwrap_err();
                    log::error!("Row for {friendship_id} has an invalid event {}", err);
                    return (
                        Err(sqlx::Error::Decode(Box::new(err))),
                        transaction_to_return,
                    );
                }

                let history = FriendshipHistory {
                    friendship_id,
                    event: event.unwrap(),
                    acting_user: row.try_get("acting_user").unwrap(),
                    timestamp: row.try_get("timestamp").unwrap(),
                    metadata: row.try_get("metadata").unwrap(),
                };
                (Ok(Some(history)), transaction_to_return)
            }
            Err(err) => match err {
                Error::RowNotFound => (Ok(None), transaction_to_return),
                _ => (Err(err), transaction_to_return),
            },
        }
    }

    /// Query the history of `request` events for a friendship.
    /// If `with_metadata` is set to true, only rows with non-empty metadata be returned.
    /// If set to false, all rows will be returned.
    pub async fn get_friendship_request_event_history<'a>(
        &'a self,
        friendship_id: Uuid,
        with_metadata: bool,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Vec<FriendshipHistory>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    ) {
        let executor = self.get_executor(transaction);

        let with_metadata_only = " AND metadata IS NOT NULL";

        // Build query
        let mut query =
            "SELECT * FROM friendship_history WHERE friendship_id = $1 AND event = '\"request\"'"
                .to_owned();

        // And metadata not null clause
        if with_metadata {
            query.push_str(with_metadata_only);
        }

        // Order by clause
        query.push_str(" ORDER BY timestamp DESC");

        let query = sqlx::query(&query).bind(friendship_id);

        let (res, resulting_executor) = DatabaseComponent::fetch_all(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match res {
            Ok(rows) => {
                let response = Ok(rows
                    .iter()
                    .map(|row| FriendshipHistory {
                        friendship_id: row.try_get("friendship_id").unwrap(),
                        event: FriendshipEvent::REQUEST,
                        acting_user: row.try_get("acting_user").unwrap(),
                        timestamp: row.try_get("timestamp").unwrap(),
                        metadata: row.try_get("metadata").unwrap(),
                    })
                    .collect::<Vec<FriendshipHistory>>());
                (response, transaction_to_return)
            }
            Err(err) => match err {
                Error::RowNotFound => (Ok(vec![]), transaction_to_return),
                _ => (Err(err), transaction_to_return),
            },
        }
    }

    fn get_executor<'a>(&'a self, transaction: Option<Transaction<'a, Postgres>>) -> Executor<'a> {
        transaction.map_or_else(
            || Executor::Pool(DatabaseComponent::get_connection(&self.db_connection)),
            Executor::Transaction,
        )
    }
}
