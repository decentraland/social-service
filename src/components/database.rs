use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::prelude::*;

use super::configuration::Database as DatabaseConfig;

pub struct DatabaseComponent {
    db_host: String,
    db_user: String,
    db_password: String,
    db_name: String,
    pub db_connection: Option<DatabaseConnection>,
}

impl DatabaseComponent {
    pub fn new(db_config: &DatabaseConfig) -> Self {
        Self {
            db_host: db_config.host.clone(),
            db_user: db_config.user.clone(),
            db_password: db_config.password.clone(),
            db_name: db_config.name.clone(),
            db_connection: None,
        }
    }

    pub async fn run(&mut self) -> Result<(), DbErr> {
        if self.db_connection.is_none() {
            let url = format!(
                "postgres://{}:{}@{}/{}",
                self.db_user, self.db_password, self.db_host, self.db_name
            );
            log::debug!("DB URL: {}", url);
            log::debug!("DB creds: {}-{}", self.db_user, self.db_password);
            let db_connection = match Database::connect(url).await {
                Ok(db) => db,
                Err(err) => {
                    log::debug!("Error on connecting to DB: {:?}", err);
                    panic!("Unable to connect to DB")
                }
            };

            self.db_connection = Some(db_connection);

            Ok(())
        } else {
            log::debug!("DB Connection is already set.");
            Ok(())
        }
    }
}
