use sea_orm::{prelude::Uuid, ConnectionTrait, DatabaseConnection, DbErr};
use std::sync::Arc;

use crate::components::database::DatabaseComponent;

const TABLE: &str = "friendships";

#[derive(Clone)]
pub struct FriendshipsRepository {
    db_connection: Arc<Option<DatabaseConnection>>,
}

pub struct Frienship {
    pub id: Uuid,
    pub address_1: String,
    pub address_2: String,
    pub is_active: bool,
}

impl FriendshipsRepository {
    pub fn new(db: Arc<Option<DatabaseConnection>>) -> Self {
        Self { db_connection: db }
    }

    // Example
    pub async fn create_new_friendships(&self, addresses: (&str, &str)) -> Result<(), DbErr> {
        let (address1, address2) = addresses;
        let query = DatabaseComponent::get_statement(
            format!("INSERT INTO {} (address1, address2) VALUES ($1, $2)", TABLE).as_str(),
            vec![address1.into(), address2.into()],
        );
        match self
            .db_connection
            .as_ref()
            .as_ref()
            .unwrap()
            .execute(query)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn get(&self, addresses: (&str, &str)) -> Result<Option<Frienship>, DbErr> {
        let (address1, address2) = addresses;
        let query = DatabaseComponent::get_statement(
            format!("SELECT * FROM {} WHERE (address1 = $1 AND address2 = $2) OR (address1 = $2 AND address2 = $1)", TABLE).as_str(),
            vec![address1.into(), address2.into()],
        );
        match self
            .db_connection
            .as_ref()
            .as_ref()
            .unwrap()
            .query_one(query)
            .await
        {
            Ok(row) => {
                if row.is_none() {
                    Ok(None)
                } else {
                    let row = row.unwrap();
                    let friendship = Frienship {
                        id: row.try_get("", "id").unwrap(),
                        address_1: row.try_get("", "address1").unwrap(),
                        address_2: row.try_get("", "address2").unwrap(),
                        is_active: row.try_get("", "is_active").unwrap(),
                    };
                    Ok(Some(friendship))
                }
            }
            Err(err) => Err(err),
        }
    }
}
