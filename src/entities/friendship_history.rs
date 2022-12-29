use std::{collections::HashMap, sync::Arc};

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
};

use super::utils::get_transaction_result_from_executor;

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

// #[automock]
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
            Err(err) => (Err(err), transaction_to_return),
        }
    }

    pub async fn get<'a>(
        &'a self,
        friendship_id: Uuid,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Option<FriendshipHistory>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    ) {
        let executor = self.get_executor(transaction);
        let query = sqlx::query("SELECT * FROM friendship_history where friendship_id = $1")
            .bind(friendship_id);

        let (res, resulting_executor) = DatabaseComponent::fetch_one(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match res {
            Ok(row) => {
                let history = FriendshipHistory {
                    friendship_id: row.try_get("friendship_id").unwrap(),
                    event: row.try_get("event").unwrap(),
                    acting_user: row.try_get("acting_user").unwrap(),
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

    fn get_executor<'a>(&'a self, transaction: Option<Transaction<'a, Postgres>>) -> Executor<'a> {
        match transaction {
            Some(transaction) => Executor::Transaction(transaction),
            None => Executor::Pool(DatabaseComponent::get_connection(&self.db_connection)),
        }
    }
}
