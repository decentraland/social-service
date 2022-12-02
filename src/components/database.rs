use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};
use sea_orm_migration::prelude::*;

use super::configuration::Database as DatabaseConfig;
use super::health::Healthy;

use crate::{
    entities::{
        friendship_history::FriendshipHistoryRepository, friendships::FriendshipsRepository,
        user_features::UserFeaturesRepository,
    },
    migrator::Migrator,
};

#[derive(Clone)]
pub struct DBRepositories {
    pub friendships: FriendshipsRepository,
    pub friendship_history: FriendshipHistoryRepository,
    pub user_features: UserFeaturesRepository,
}

#[derive(Clone)]
pub struct DatabaseComponent {
    db_host: String,
    db_user: String,
    db_password: String,
    db_name: String,
    db_connection: Arc<Option<DatabaseConnection>>,
    pub db_repos: Option<DBRepositories>,
}

impl DatabaseComponent {
    pub fn new(db_config: &DatabaseConfig) -> Self {
        Self {
            db_host: db_config.host.clone(),
            db_user: db_config.user.clone(),
            db_password: db_config.password.clone(),
            db_name: db_config.name.clone(),
            db_connection: Arc::new(None),
            db_repos: None,
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

            db_connection
                .execute(Statement::from_string(
                    DatabaseBackend::Postgres,
                    format!("CREATE EXTENSION IF NOT EXISTS \"{}\"", "uuid-ossp").to_string(),
                ))
                .await?;

            log::debug!("Running Database migrations...");

            // Just runs the pending migrations
            Migrator::up(&db_connection, None).await?;

            log::debug!("Migrations executed!");

            self.db_connection = Arc::new(Some(db_connection));
            self.db_repos = Some(DBRepositories {
                friendships: FriendshipsRepository::new(self.db_connection.clone()),
                friendship_history: FriendshipHistoryRepository::new(self.db_connection.clone()),
                user_features: UserFeaturesRepository::new(self.db_connection.clone()),
            });

            Ok(())
        } else {
            log::debug!("DB Connection is already set.");
            Ok(())
        }
    }

    pub fn get_statement<V: IntoIterator<Item = Value>>(query: &str, values: V) -> Statement {
        let sql = format!(r#"{}"#, query);
        Statement::from_sql_and_values(DatabaseBackend::Postgres, &sql, values)
    }

    pub fn is_connected(&self) -> bool {
        self.db_connection.is_some()
    }
}

#[async_trait]
impl Healthy for DatabaseComponent {
    async fn is_healthy(&self) -> bool {
        match self
            .db_connection
            .as_ref()
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
