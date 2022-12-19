use sqlx::{types::Uuid, Error, Row};
use std::{fmt, sync::Arc};

use crate::{
    components::database::{DBConnection, DatabaseComponent},
    generate_uuid_v4,
};

#[cfg_attr(test, faux::create)]
#[derive(Clone)]
pub struct FriendshipsRepository {
    db_connection: Arc<Option<DBConnection>>,
}

pub struct Friendship {
    pub id: Uuid,
    pub address_1: String,
    pub address_2: String,
    pub is_active: bool,
}

#[cfg_attr(test, faux::methods)]
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

#[cfg_attr(test, faux::methods)]
impl FriendshipsRepository {
    pub fn new(db: Arc<Option<DBConnection>>) -> Self {
        Self { db_connection: db }
    }

    fn get_db_connection(&self) -> Arc<Option<DBConnection>> {
        self.db_connection.clone()
    }

    // Example
    pub async fn create_new_friendships(&self, addresses: (&str, &str)) -> Result<(), sqlx::Error> {
        let (address1, address2) = addresses;
        let db_conn = DatabaseComponent::get_connection(&self.db_connection);
        match sqlx::query("INSERT INTO friendships(id, address_1, address_2) VALUES($1,$2, $3);")
            .bind(Uuid::parse_str(generate_uuid_v4().as_str()).unwrap())
            .bind(address1)
            .bind(address2)
            .execute(db_conn)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn get(&self, addresses: (&str, &str)) -> Result<Option<Friendship>, sqlx::Error> {
        let (address1, address2) = addresses;
        let db_conn = DatabaseComponent::get_connection(&self.db_connection);
        match sqlx::query(
            "SELECT * FROM friendships WHERE (address_1 = $1 AND address_2 = $2) OR (address_1 = $3 AND address_2 = $4)"
        )
        .bind(address1)
        .bind(address2)
        .bind(address2)
        .bind(address1)
        .fetch_one(db_conn).await
        {
            Ok(row) => {
                let friendship = Friendship {
                    id: row.try_get("id").unwrap(),
                    address_1: row.try_get("address_1").unwrap(),
                    address_2: row.try_get("address_2").unwrap(),
                    is_active: row.try_get("is_active").unwrap(),
                };
                Ok(Some(friendship))
            }
            Err(err) => match err {
                Error::RowNotFound => Ok(None),
                _ => Err(err),
            },
        }
    }

    /// Fetches the friendships of a given user
    /// if include inactive is true, this will also return all addresses for users
    /// that this user has been friends in the past]
    #[tracing::instrument(name = "Get user friends from DB")]
    pub async fn get_user_friends(
        &self,
        address: &str,
        include_inactive: bool,
    ) -> Result<Vec<Friendship>, sqlx::Error> {
        let db_conn = DatabaseComponent::get_connection(&self.db_connection);
        let active_only_clause = " AND is_active";

        let mut query =
            "SELECT * FROM friendships WHERE (address_1 = $1) OR (address_2 = $1)".to_owned();

        if include_inactive {
            query.push_str(active_only_clause);
        }

        match sqlx::query(&query).bind(address).fetch_all(db_conn).await {
            Ok(rows) => Ok(rows
                .iter()
                .map(|row| -> Friendship {
                    let friendship = Friendship {
                        id: row.try_get("id").unwrap(),
                        address_1: row.try_get("address_1").unwrap(),
                        address_2: row.try_get("address_2").unwrap(),
                        is_active: row.try_get("is_active").unwrap(),
                    };
                    friendship
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
