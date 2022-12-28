use async_trait::async_trait;
use mockall::automock;
use sqlx::{query::Query, types::Uuid, Error, Postgres, Row, Transaction};
use std::{fmt, sync::Arc};

use crate::{
    components::database::{DBConnection, DatabaseComponent},
    generate_uuid_v4,
};

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

    async fn get<'a>(
        &self,
        addresses: (&'a str, &'a str),
        transaction: Option<Transaction<'a, Postgres>>,
    ) -> (
        Result<Option<Friendship>, sqlx::Error>,
        Option<Transaction<'a, Postgres>>,
    );

    async fn get_user_friends<'a>(
        &self,
        address: &str,
        include_inactive: bool,
        transaction: &'a mut Option<Transaction<'a, Postgres>>,
    ) -> Result<Vec<Friendship>, sqlx::Error>;
}

// #[automock]
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
        let db_conn = DatabaseComponent::get_connection(&self.db_connection);
        let query = self.create_new_friendships_query(addresses);
        let (res, transaction) =
            DatabaseComponent::execute_query(query, transaction, db_conn).await;
        match res {
            Ok(_) => (Ok(()), transaction),
            Err(err) => (Err(err), transaction),
        }
    }

    async fn get<'a>(
        &self,
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
        .bind(&address1)
        .bind(&address2)
        .bind(&address2)
        .bind(&address1);

        let db_conn = DatabaseComponent::get_connection(&self.db_connection);

        let (result, transaction) = DatabaseComponent::fetch_one(query, transaction, db_conn).await;

        match result {
            Ok(row) => {
                let friendship = Friendship {
                    id: row.try_get("id").unwrap(),
                    address_1: row.try_get("address_1").unwrap(),
                    address_2: row.try_get("address_2").unwrap(),
                    is_active: row.try_get("is_active").unwrap(),
                };
                (Ok(Some(friendship)), transaction)
            }
            Err(err) => match err {
                Error::RowNotFound => (Ok(None), transaction),
                _ => (Err(err), transaction),
            },
        }
    }

    /// Fetches the friendships of a given user
    /// if include inactive is true, this will also return all addresses for users
    /// that this user has been friends in the past]
    #[tracing::instrument(name = "Get user friends from DB")]
    async fn get_user_friends<'a>(
        &self,
        address: &str,
        include_inactive: bool,
        transaction: &'a mut Option<Transaction<'a, Postgres>>,
    ) -> Result<Vec<Friendship>, sqlx::Error> {
        let db_conn = DatabaseComponent::get_connection(&self.db_connection);
        let active_only_clause = " AND is_active";

        let mut query =
            "SELECT * FROM friendships WHERE (address_1 = $1) OR (address_2 = $1)".to_owned();

        if include_inactive {
            query.push_str(active_only_clause);
        }

        let query = sqlx::query(&query);

        let res = DatabaseComponent::fetch_all(query, transaction, db_conn).await;

        match res {
            Ok(rows) => Ok(rows
                .iter()
                .map(|row| -> Friendship {
                    Friendship {
                        id: row.try_get("id").unwrap(),
                        address_1: row.try_get("address_1").unwrap(),
                        address_2: row.try_get("address_2").unwrap(),
                        is_active: row.try_get("is_active").unwrap(),
                    }
                })
                .collect::<Vec<Friendship>>()),
            Err(err) => match err {
                Error::RowNotFound => Ok(vec![]),
                _ => {
                    log::error!("Couldn't fetch user {} friends, {}", address, err);
                    Err(err)
                }
            },
        }
    }
}
