use async_trait::async_trait;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};
use sea_orm_migration::prelude::*;

use super::configuration::Database as DatabaseConfig;
use super::health::Healthy;

use crate::migrator::Migrator;
use crate::{get_migration_helper, is_local, MigrationHelper};

#[derive(Clone)]
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

            let mut opts = ConnectOptions::new(url);
            // Connection Pool
            opts.min_connections(5);
            opts.max_connections(10);

            let db_connection = match Database::connect(opts).await {
                Ok(db) => db,
                Err(err) => {
                    log::debug!("Error on connecting to DB: {:?}", err);
                    panic!("Unable to connect to DB")
                }
            };

            if !is_local() {
                log::debug!("Running Database migrations...");
                // Just runs the pending migrations
                Migrator::up(&db_connection, None).await?;
                log::debug!("Migrations executed!");
            }

            self.db_connection = Some(db_connection);

            Ok(())
        } else {
            log::debug!("DB Connection is already set.");
            Ok(())
        }
    }

    pub async fn run_helper(&self) -> Result<(), DbErr> {
        let (helper, steps) = get_migration_helper();
        if self.db_connection.is_some() {
            if matches!(helper, MigrationHelper::UP) {
                Migrator::up(&self.db_connection.as_ref().unwrap(), steps).await?;
                Ok(())
            } else if matches!(helper, MigrationHelper::DOWN) {
                Migrator::down(&self.db_connection.as_ref().unwrap(), steps).await?;
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl Healthy for DatabaseComponent {
    async fn is_healthy(&self) -> bool {
        match self
            .db_connection
            .as_ref()
            .unwrap()
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT CURRENT_TIMESTAMP;".to_owned(),
            ))
            .await
        {
            Ok(result) => result.is_some(),
            Err(_) => false,
        }
    }
}
