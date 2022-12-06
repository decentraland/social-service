use sqlx::{types::Uuid, Error, Row};
use std::sync::Arc;

use crate::components::database::{DBConnection, DatabaseComponent};

#[derive(Clone)]
pub struct FriendshipsRepository {
    db_connection: Arc<Option<DBConnection>>,
}

pub struct Frienship {
    pub id: Uuid,
    pub address_1: String,
    pub address_2: String,
    pub is_active: bool,
}

impl FriendshipsRepository {
    pub fn new(db: Arc<Option<DBConnection>>) -> Self {
        Self { db_connection: db }
    }

    // Example
    pub async fn create_new_friendships(&self, addresses: (&str, &str)) -> Result<(), sqlx::Error> {
        let (address1, address2) = addresses;
        let db_conn = DatabaseComponent::get_connection(&self.db_connection);
        match sqlx::query("INSERT INTO friendships(address_1, address_2) VALUES($1,$2);")
            .bind(address1)
            .bind(address2)
            .execute(db_conn)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn get(&self, addresses: (&str, &str)) -> Result<Option<Frienship>, sqlx::Error> {
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
                let friendship = Frienship {
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
}
