use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use sqlx::{types::Uuid, Error, FromRow, Postgres, Row, Transaction};
use std::{fmt, pin::Pin, sync::Arc};

use super::queries::MUTUALS_FRIENDS_QUERY;

use crate::{
    components::database::{DBConnection, DatabaseComponent, Executor},
    generate_uuid_v4,
};

use super::utils::get_transaction_result_from_executor;

#[derive(Default, FromRow)]
pub struct Friendship {
    pub id: Uuid,
    pub address_1: String,
    pub address_2: String,
    pub is_active: bool,
    pub synapse_room_id: String,
}

#[derive(FromRow)]
pub struct UserEntity {
    pub address: String,
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

    async fn create_new_friendships(
        &self,
        addresses: (&str, &str),
        is_active: bool,
        synapse_room_id: &str,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<Uuid, sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
    );

    async fn get_friendship(
        &self,
        addresses: (&str, &str),
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<Option<Friendship>, sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
    );

    async fn get_user_friends(
        &self,
        address: &str,
        only_active: bool,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<Vec<Friendship>, sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
    );

    async fn get_user_friends_stream(
        &self,
        address: &str,
        only_active: bool,
    ) -> Result<Pin<Box<dyn Stream<Item = Friendship> + Send>>, sqlx::Error>;

    async fn get_mutual_friends_stream<'a>(
        &'a self,
        address_1: String,
        address_2: String,
    ) -> Result<Pin<Box<dyn Stream<Item = UserEntity> + Send>>, sqlx::Error>;

    async fn update_friendship_status(
        &self,
        friendship_id: &Uuid,
        is_active: bool,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<(), sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
    );

    async fn get_mutual_friends(
        &self,
        address_1: &str,
        address_2: &str,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<Vec<String>, sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
    );

    fn get_executor<'a>(&self, transaction: Option<Transaction<'static, Postgres>>)
        -> Executor<'a>;
}

#[async_trait]
impl FriendshipRepositoryImplementation for FriendshipsRepository {
    fn get_db_connection(&self) -> &Arc<Option<DBConnection>> {
        &self.db_connection
    }

    async fn create_new_friendships(
        &self,
        addresses: (&str, &str),
        is_active: bool,
        synapse_room_id: &str,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<Uuid, sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
    ) {
        // The addresses are lexicographicly sorted to ensure that the friendship tuple is unique
        let (address1, address2) = sort_addresses(addresses);

        let id = Uuid::parse_str(generate_uuid_v4().as_str()).unwrap();

        let query = sqlx::query(
            "INSERT INTO friendships(id, address_1, address_2, is_active, synapse_room_id) VALUES($1, $2, $3, $4, $5);",
        )
        .bind(id)
        .bind(address1)
        .bind(address2)
        .bind(is_active)
        .bind(synapse_room_id);

        let executor = self.get_executor(transaction);

        let (res, resulting_executor) = DatabaseComponent::execute_query(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match res {
            Ok(_) => (Ok(id), transaction_to_return),
            Err(err) => (Err(err), transaction_to_return),
        }
    }

    async fn get_friendship(
        &self,
        addresses: (&str, &str),
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<Option<Friendship>, sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
    ) {
        let (address1, address2) = addresses;

        let query = sqlx::query(
            "SELECT * FROM friendships WHERE (LOWER(address_1) = LOWER($1) AND LOWER(address_2) = LOWER($2)) OR (LOWER(address_1) = LOWER($2) AND LOWER(address_2) = LOWER($1))"
        )
        .bind(address1)
        .bind(address2);

        let executor = self.get_executor(transaction);

        let (result, resulting_executor) = DatabaseComponent::fetch_one(query, executor).await;

        let transaction_to_return = get_transaction_result_from_executor(resulting_executor);

        match result {
            Ok(row) => {
                let friendship = Friendship::from_row(&row).expect("to be a friendship");
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
    async fn get_user_friends(
        &self,
        address: &str,
        only_active: bool,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<Vec<Friendship>, sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
    ) {
        let active_only_clause = " AND is_active";

        let mut query =
            "SELECT * FROM friendships WHERE (LOWER(address_1) = LOWER($1) OR LOWER(address_2) = LOWER($1))"
                .to_owned();

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
                        Friendship::from_row(row).expect("to be a friendship")
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

    /// If `only_active` is set to true, only the current friends will be returned.
    /// If set to false, all past and current friendships will be returned.
    #[tracing::instrument(name = "Get user friends from DB stream")]
    async fn get_user_friends_stream(
        &self,
        address: &str,
        only_active: bool,
    ) -> Result<Pin<Box<dyn Stream<Item = Friendship> + Send>>, sqlx::Error> {
        let active = "SELECT * FROM friendships WHERE (LOWER(address_1) = LOWER($1) OR LOWER(address_2) = LOWER($1)) AND is_active;";
        let inactive =
            "SELECT * FROM friendships WHERE (LOWER(address_1) = LOWER($1) OR LOWER(address_2) = LOWER($1));";

        let query = if only_active { active } else { inactive };

        let query = sqlx::query(query).bind(address.to_string());

        let pool = DatabaseComponent::get_connection(&self.db_connection).clone();

        let response = DatabaseComponent::fetch_stream(query, pool);
        let friends_stream = response.filter_map(|row| async move {
            match row {
                Ok(row) => {
                    let friendship = Friendship::from_row(&row).expect("to be a friendship");
                    Some(friendship)
                }
                Err(err) => {
                    log::error!("Couldn't stream fetch user friends, {}", err);
                    None
                }
            }
        });
        Ok(Box::pin(friends_stream))
    }

    #[tracing::instrument(name = "Get mutual friends from DB stream")]
    async fn get_mutual_friends_stream<'a>(
        &'a self,
        address_1: String,
        address_2: String,
    ) -> Result<Pin<Box<dyn Stream<Item = UserEntity> + Send>>, sqlx::Error> {
        let query: &str = MUTUALS_FRIENDS_QUERY;

        let query = sqlx::query(query).bind(address_1).bind(address_2);

        let pool = DatabaseComponent::get_connection(&self.db_connection).clone();

        let response = DatabaseComponent::fetch_stream(query, pool);
        let mutual_friends_stream = response.filter_map(|row| async move {
            match row {
                Ok(row) => {
                    let user = UserEntity::from_row(&row).expect("to be a user");
                    Some(user)
                }
                Err(err) => {
                    log::error!("Couldn't stream fetch mutual friends, {}", err);
                    None
                }
            }
        });
        Ok(Box::pin(mutual_friends_stream))
    }

    #[tracing::instrument(name = "Get mutual user friends from DB")]
    async fn get_mutual_friends(
        &self,
        address_1: &str,
        address_2: &str,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<Vec<String>, sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
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

    async fn update_friendship_status(
        &self,
        friendship_id: &Uuid,
        is_active: bool,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> (
        Result<(), sqlx::Error>,
        Option<Transaction<'static, Postgres>>,
    ) {
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

    fn get_executor<'a>(
        &self,
        transaction: Option<Transaction<'static, Postgres>>,
    ) -> Executor<'a> {
        match transaction {
            Some(transaction) => Executor::Transaction(transaction),
            None => {
                // choose to Clone because it's cheap and the pool use an Arc internally
                let conn = DatabaseComponent::get_connection(&self.db_connection).clone();
                Executor::Pool(conn)
            }
        }
    }
}

fn sort_addresses<'a>(addresses: (&'a str, &'a str)) -> (&'a str, &'a str) {
    let (address1, address2) = addresses;

    if address1 < address2 {
        addresses
    } else {
        (address2, address1)
    }
}
