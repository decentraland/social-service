use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr};
use std::sync::Arc;

use crate::components::database::DatabaseComponent;

const TABLE: &str = "friendships";

#[derive(Clone)]
pub struct FriendshipsRepository {
    db_connection: Arc<Option<DatabaseConnection>>,
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
}
