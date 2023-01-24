use async_trait::async_trait;
use sqlx::{types::Uuid, Error, Postgres, Row, Transaction};
use std::{fmt, sync::Arc};

use super::queries::MUTUALS_FRIENDS_QUERY;
use crate::{
    components::database::{DBConnection, DatabaseComponent, Executor},
    generate_uuid_v4,
};

use super::utils::get_transaction_result_from_executor;

pub struct Friendship {
    pub id: Uuid,
    pub address_1: String,
    pub address_2: String,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct FriendshipsRepository {
    db_connection: Arc<Option<DBConnection>>,
}

impl FriendshipsRepository {
    pub fn new(db: Arc<Option<DBConnection>>) -> Self {
        Self { db_connection: db }
    }
}

impl fmt::Debug for FriendshipsRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FriendshipsRepository")
            .field(
                "db_connection has value",
                &self.get_db_connection().is_some(),
            )
            .finish()
    }
}

#[async_trait]
pub trait FriendshipRepositoryImplementation {
    fn get_db_connection(&self) -> &Arc<Option<DBConnection>>;

    async fn create_new_friendships<'a>(
        &'a self,
        addresses: (&'a str, &'a str),
        is_active: bool,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (Result<Uuid, sqlx::Error>, Option<Transaction<'a, Postgres>>);

    async fn get_friendship<'a>(
        &'a self,
        addresses: (&'a str, &'a str),
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Option<Friendship>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    );

    async fn get_user_friends<'a>(
        &'a self,
        address: &'a str,
        only_active: bool,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Vec<Friendship>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    );

    async fn update_friendship_status<'a>(
        &'a self,
        friendship_id: &'a Uuid,
        is_active: bool,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (Result<(), sqlx::Error>, Option<Transaction<'a, Postgres>>);

    async fn get_mutual_friends<'a>(
        &'a self,
        address_1: &'a str,
        address_2: &'a str,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Vec<String>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    );

    fn get_executor<'a>(&'a self, transaction: Option<Transaction<'a, Postgres>>) -> Executor<'a>;
}

#[async_trait]
impl FriendshipRepositoryImplementation for FriendshipsRepository {
    fn get_db_connection(&self) -> &Arc<Option<DBConnection>> {
        &self.db_connection
    }

    async fn create_new_friendships<'a>(
        &'a self,
        addresses: (&'a str, &'a str),
        is_active: bool,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (Result<Uuid, sqlx::Error>, Option<Transaction<'a, Postgres>>) {
        let (address1, address2) = addresses;

        let id = Uuid::parse_str(generate_uuid_v4().as_str()).unwrap();

        let query = sqlx::query(
            "INSERT INTO friendships(id, address_1, address_2, is_active) VALUES($1, $2, $3, $4);",
        )
        .bind(id)
        .bind(address1)
        .bind(address2)
        .bind(is_active);

        let executor = self.get_executor(transaction);

        let (res, resulting_executor) = DatabaseComponent::execute_query(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match res {
            Ok(_) => (Ok(id), transaction_to_return),
            Err(err) => (Err(err), transaction_to_return),
        }
    }

    async fn get_friendship<'a>(
        &'a self,
        addresses: (&'a str, &'a str),
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Option<Friendship>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    ) {
        let (address1, address2) = addresses;

        let query = sqlx::query(
            "SELECT * FROM friendships WHERE (address_1 = $1 AND address_2 = $2) OR (address_1 = $3 AND address_2 = $4)"
        )
        .bind(address1.to_string())
        .bind(address2.to_string())
        .bind(address2.to_string())
        .bind(address1.to_string());

        let executor = self.get_executor(transaction);

        let (result, resulting_executor) = DatabaseComponent::fetch_one(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match result {
            Ok(row) => {
                let friendship = Friendship {
                    id: row.try_get("id").unwrap(),
                    address_1: row.try_get("address_1").unwrap(),
                    address_2: row.try_get("address_2").unwrap(),
                    is_active: row.try_get("is_active").unwrap(),
                };
                (Ok(Some(friendship)), transaction_to_return)
            }
            Err(err) => match err {
                Error::RowNotFound => (Ok(None), transaction_to_return),
                _ => (Err(err), transaction_to_return),
            },
        }
    }

    /// Fetches the friendships of a given user.
    /// If `only_active` is set to true, only the current friends will be returned.
    /// If set to false, all past and current friendships will be returned.
    #[tracing::instrument(name = "Get user friends from DB")]
    async fn get_user_friends<'a>(
        &'a self,
        address: &'a str,
        only_active: bool,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Vec<Friendship>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    ) {
        let active_only_clause = " AND is_active";

        let mut query =
            "SELECT * FROM friendships WHERE (address_1 = $1) OR (address_2 = $1)".to_owned();

        if only_active {
            query.push_str(active_only_clause);
        }

        let query = sqlx::query(&query).bind(address);

        let executor = self.get_executor(transaction);

        let (res, resulting_executor) = DatabaseComponent::fetch_all(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match res {
            Ok(rows) => {
                let response = Ok(rows
                    .iter()
                    .map(|row| -> Friendship {
                        Friendship {
                            id: row.try_get("id").unwrap(),
                            address_1: row.try_get("address_1").unwrap(),
                            address_2: row.try_get("address_2").unwrap(),
                            is_active: row.try_get("is_active").unwrap(),
                        }
                    })
                    .collect::<Vec<Friendship>>());
                (response, transaction_to_return)
            }
            Err(err) => match err {
                Error::RowNotFound => (Ok(vec![]), transaction_to_return),
                _ => {
                    log::error!("Couldn't fetch user {} friends, {}", address, err);
                    (Err(err), transaction_to_return)
                }
            },
        }
    }

    #[tracing::instrument(name = "Get mutual user friends from DB")]
    async fn get_mutual_friends<'a>(
        &'a self,
        address_1: &'a str,
        address_2: &'a str,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Vec<String>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    ) {
        let query = MUTUALS_FRIENDS_QUERY.to_string();

        let query = sqlx::query(&query).bind(address_1).bind(address_2);

        let executor = self.get_executor(transaction);

        let (res, resulting_executor) = DatabaseComponent::fetch_all(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match res {
            Ok(rows) => {
                let response = Ok(rows
                    .iter()
                    .map(|row| row.try_get("address").unwrap())
                    .collect::<Vec<String>>());
                (response, transaction_to_return)
            }
            Err(err) => match err {
                Error::RowNotFound => (Ok(vec![]), transaction_to_return),
                _ => {
                    log::error!(
                        "Couldn't fetch user {} mutual friends with {}, {}",
                        address_1,
                        address_2,
                        err
                    );
                    (Err(err), transaction_to_return)
                }
            },
        }
    }

    async fn update_friendship_status<'a>(
        &'a self,
        friendship_id: &'a Uuid,
        is_active: bool,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (Result<(), sqlx::Error>, Option<Transaction<'a, Postgres>>) {
        let query = sqlx::query("UPDATE friendships SET is_active = $1 WHERE id = $2")
            .bind(is_active)
            .bind(friendship_id);

        let executor = self.get_executor(transaction);

        let (res, resulting_executor) = DatabaseComponent::execute_query(query, executor).await;
        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match res {
            Ok(_) => (Ok(()), transaction_to_return),
            Err(err) => (Err(err), transaction_to_return),
        }
    }

    fn get_executor<'a>(&'a self, transaction: Option<Transaction<'a, Postgres>>) -> Executor<'a> {
        match transaction {
            Some(transaction) => Executor::Transaction(transaction),
            None => Executor::Pool(DatabaseComponent::get_connection(&self.db_connection)),
        }
    }
}
