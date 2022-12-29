use async_trait::async_trait;
use sqlx::{query::Query, types::Uuid, Error, Postgres, Row, Transaction};
use std::{fmt, sync::Arc};

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

    fn create_new_friendships_query<'a>(
        &self,
        addresses: (&'a str, &'a str),
    ) -> Query<'a, Postgres, sqlx::postgres::PgArguments>;

    async fn create_new_friendships<'a>(
        &'a self,
        addresses: (&'a str, &'a str),
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (Result<(), sqlx::Error>, Option<Transaction<'a, Postgres>>);

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
        include_inactive: bool,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Vec<Friendship>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    );

    fn get_executor<'a>(&'a self, transaction: Option<Transaction<'a, Postgres>>) -> Executor<'a>;
}

#[async_trait]
impl FriendshipRepositoryImplementation for FriendshipsRepository {
    fn get_db_connection(&self) -> &Arc<Option<DBConnection>> {
        &self.db_connection
    }

    fn create_new_friendships_query<'a>(
        &self,
        addresses: (&'a str, &'a str),
    ) -> Query<'a, Postgres, sqlx::postgres::PgArguments> {
        let (address1, address2) = addresses;
        sqlx::query("INSERT INTO friendships(id, address_1, address_2) VALUES($1,$2, $3);")
            .bind(Uuid::parse_str(generate_uuid_v4().as_str()).unwrap())
            .bind(address1)
            .bind(address2)
    }

    async fn create_new_friendships<'a>(
        &'a self,
        addresses: (&'a str, &'a str),
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (Result<(), sqlx::Error>, Option<Transaction<'a, Postgres>>) {
        let query = self.create_new_friendships_query(addresses);

        let executor = self.get_executor(transaction);

        let (res, resulting_executor) = DatabaseComponent::execute_query(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match res {
            Ok(_) => (Ok(()), transaction_to_return),
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

    /// Fetches the friendships of a given user
    /// if include inactive is true, this will also return all addresses for users
    /// that this user has been friends in the past]
    #[tracing::instrument(name = "Get user friends from DB")]
    async fn get_user_friends<'a>(
        &'a self,
        address: &'a str,
        include_inactive: bool,
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Vec<Friendship>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    ) {
        let active_only_clause = " AND is_active";

        let mut query =
            "SELECT * FROM friendships WHERE (address_1 = $1) OR (address_2 = $1)".to_owned();

        if include_inactive {
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

    fn get_executor<'a>(&'a self, transaction: Option<Transaction<'a, Postgres>>) -> Executor<'a> {
        match transaction {
            Some(transaction) => Executor::Transaction(transaction),
            None => Executor::Pool(DatabaseComponent::get_connection(&self.db_connection)),
        }
    }
}
