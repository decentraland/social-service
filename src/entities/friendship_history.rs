use std::sync::Arc;

use chrono::NaiveDateTime;
use mockall::predicate::*;
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::Postgres,
    query::Query,
    types::{Json, Uuid},
    Error, FromRow, Row, Transaction,
};

use crate::{
    components::database::{DBConnection, DatabaseComponent, Executor},
    domain::friendship_event::FriendshipEvent,
    entities::queries::USER_REQUESTS_QUERY,
    entities::utils::get_transaction_result_from_executor,
    generate_uuid_v4,
};

#[derive(Clone)]
pub struct FriendshipHistoryRepository {
    db_connection: Arc<Option<DBConnection>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FriendshipMetadata {
    pub message: Option<String>,
    pub synapse_room_id: Option<String>,
    pub migrated_from_synapse: Option<bool>,
}

pub struct FriendshipHistory {
    pub friendship_id: Uuid,
    pub event: FriendshipEvent,
    pub acting_user: String,
    pub timestamp: NaiveDateTime,
    pub metadata: Option<Json<FriendshipMetadata>>,
}

#[derive(FromRow)]
pub struct FriendshipRequestEvent {
    pub address_1: String,
    pub address_2: String,
    pub acting_user: String,
    pub timestamp: NaiveDateTime,
    pub metadata: Option<Json<FriendshipMetadata>>,
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
        metadata: Option<Json<FriendshipMetadata>>,
    ) -> Query<'a, Postgres, sqlx::postgres::PgArguments> {
        sqlx::query(
                "INSERT INTO friendship_history (id, friendship_id, event, acting_user, metadata) VALUES ($1, $2, $3, $4, $5)",
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
        metadata: Option<Json<FriendshipMetadata>>,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<(), sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
    ) {
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

    pub async fn get_last_history_for_friendship(
        &self,
        friendship_id: Uuid,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<Option<FriendshipHistory>, sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
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
                    metadata: row.try_get("metadata").unwrap_or(None),
                };
                (Ok(Some(history)), transaction_to_return)
            }
            Err(err) => match err {
                Error::RowNotFound => (Ok(None), transaction_to_return),
                _ => (Err(err), transaction_to_return),
            },
        }
    }

    /// Fetches the pending request events of the given user.
    pub async fn get_user_pending_request_events(
        &self,
        address: &str,
    ) -> Result<Vec<FriendshipRequestEvent>, sqlx::Error> {
        let query = USER_REQUESTS_QUERY.to_string();

        let query = sqlx::query(&query).bind(address);

        let executor = self.get_executor(None);

        let (res, _) = DatabaseComponent::fetch_all(query, executor).await;

        match res {
            Ok(rows) => {
                let response = Ok(rows
                    .iter()
                    .map(|row| -> FriendshipRequestEvent {
                        FriendshipRequestEvent::from_row(row)
                            .expect("to be a friendship request event")
                    })
                    .collect::<Vec<FriendshipRequestEvent>>());
                response
            }
            Err(Error::RowNotFound) => Ok(vec![]),
            Err(err) => {
                log::error!("Couldn't fetch user {} requests, {}", address, err);
                Err(err)
            }
        }
    }

    fn get_executor(
        &self,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> Executor<'static> {
        transaction.map_or_else(
            || Executor::Pool(DatabaseComponent::get_connection(&self.db_connection).clone()), // choose to Clone because it's cheap and the pool use an Arc internally
            Executor::Transaction,
        )
    }
}
