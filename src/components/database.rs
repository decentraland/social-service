use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use chrono::DateTime;

use futures_util::stream::BoxStream;
use log::LevelFilter;
use mockall::automock;
use sqlx::{
    postgres::{PgArguments, PgConnectOptions, PgPoolOptions, PgQueryResult, PgRow},
    query::Query,
    ConnectOptions, Error, Pool, Postgres, Row, Transaction,
};

use super::configuration::Database as DatabaseConfig;
use super::health::Healthy;

use crate::entities::{
    friendship_history::FriendshipHistoryRepository, friendships::FriendshipsRepository,
    user_features::UserFeaturesRepository,
};

pub type DBConnection = Pool<Postgres>;

#[derive(Clone)]
pub struct DBRepositories {
    pub friendships: FriendshipsRepository,
    pub friendship_history: FriendshipHistoryRepository,
    pub user_features: UserFeaturesRepository,
}

impl DBRepositories {
    pub fn new(
        friendships: FriendshipsRepository,
        friendship_history: FriendshipHistoryRepository,
        user_features: UserFeaturesRepository,
    ) -> Self {
        Self {
            friendships,
            friendship_history,
            user_features,
        }
    }
}

pub enum Executor<'a> {
    Transaction(Transaction<'a, Postgres>),
    Pool(Pool<Postgres>),
}

#[derive(Clone)]
pub struct DatabaseComponent {
    db_host: String,
    db_user: String,
    db_password: String,
    db_name: String,
    db_connection: Arc<Option<DBConnection>>,
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

    pub fn get_connection(db_connection: &Arc<Option<DBConnection>>) -> &DBConnection {
        db_connection.as_ref().as_ref().unwrap()
    }

    pub async fn execute_query<'a>(
        query: Query<'_, Postgres, PgArguments>,
        executor: Executor<'a>,
    ) -> (Result<PgQueryResult, Error>, Option<Executor<'a>>) {
        match executor {
            Executor::Transaction(mut transaction) => (
                query.execute(&mut transaction).await,
                Some(Executor::Transaction(transaction)),
            ),
            // we don't return the pool because the connection was consumed
            Executor::Pool(pool) => (query.execute(&pool).await, None),
        }
    }

    pub async fn fetch_one<'a>(
        query: Query<'_, Postgres, PgArguments>,
        executor: Executor<'a>,
    ) -> (Result<PgRow, Error>, Option<Executor<'a>>) {
        match executor {
            Executor::Transaction(mut transaction) => (
                query.fetch_one(&mut transaction).await,
                Some(Executor::Transaction(transaction)),
            ),
            // we don't return the pool because the connection was consumed
            Executor::Pool(pool) => (query.fetch_one(&pool).await, None),
        }
    }

    pub async fn fetch_all<'a>(
        query: Query<'_, Postgres, PgArguments>,
        executor: Executor<'a>,
    ) -> (Result<Vec<PgRow>, Error>, Option<Executor<'a>>) {
        match executor {
            Executor::Transaction(mut transaction) => (
                query.fetch_all(&mut transaction).await,
                Some(Executor::Transaction(transaction)),
            ),
            // we don't return the pool because the connection was consumed
            Executor::Pool(pool) => (query.fetch_all(&pool).await, None),
        }
    }

    pub fn fetch_stream(
        query: Query<'_, Postgres, PgArguments>,
        pool: Pool<Postgres>,
    ) -> BoxStream<'_, Result<PgRow, Error>> {
        query.fetch(&pool)
    }
}

#[async_trait]
pub trait DatabaseComponentImplementation {
    fn get_repos(&self) -> &Option<DBRepositories>;
    async fn run(&mut self) -> Result<(), sqlx::Error>;
    fn is_connected(&self) -> bool;
    async fn start_transaction(&self) -> Result<Transaction<'static, Postgres>, Error>;

    async fn close(&self);
}

#[automock]
#[async_trait]
impl DatabaseComponentImplementation for DatabaseComponent {
    fn get_repos(&self) -> &Option<DBRepositories> {
        &self.db_repos
    }

    async fn run(&mut self) -> Result<(), sqlx::Error> {
        if !self.is_connected() {
            let url = format!(
                "postgres://{}:{}@{}/{}",
                self.db_user, self.db_password, self.db_host, self.db_name
            );
            log::info!("DB URL: {}", url);

            let pool = PgPoolOptions::new().min_connections(5).max_connections(10);

            let options = PgConnectOptions::from_str(url.as_str())
                .expect("Unable to parse Database URL")
                .log_statements(LevelFilter::Debug) // Only log queries when running in debug log level
                .clone();

            let db_connection = match pool.connect_with(options).await {
                Ok(db) => db,
                Err(err) => {
                    log::error!("Error on connecting to DB: {:?}", err);
                    panic!("Unable to connect to DB")
                }
            };

            log::info!("Running Database migrations...");

            // Just runs the pending migrations
            if let Err(err) = sqlx::migrate!("./migrations").run(&db_connection).await {
                log::error!("Error on running DB Migrations. Err: {:?}", err);
                panic!("Unable to run pending migrations")
            } else {
                log::info!("Migrations executed!");
            }

            self.db_connection = Arc::new(Some(db_connection));
            self.db_repos = Some(DBRepositories::new(
                FriendshipsRepository::new(self.db_connection.clone()),
                FriendshipHistoryRepository::new(self.db_connection.clone()),
                UserFeaturesRepository::new(self.db_connection.clone()),
            ));

            Ok(())
        } else {
            log::warn!("DB Connection is already set.");
            Ok(())
        }
    }

    fn is_connected(&self) -> bool {
        self.db_connection.is_some()
    }

    async fn close(&self) {
        if let Some(connection) = &self.db_connection.as_ref() {
            connection.close().await;
        }
    }

    async fn start_transaction(&self) -> Result<Transaction<'static, Postgres>, Error> {
        let db_connection = self.db_connection.as_ref().as_ref().unwrap();

        db_connection.begin().await
    }
}

#[async_trait]
impl Healthy for DatabaseComponent {
    async fn is_healthy(&self) -> bool {
        match sqlx::query("SELECT CURRENT_TIMESTAMP")
            .fetch_one(DatabaseComponent::get_connection(&self.db_connection))
            .await
        {
            Ok(result) => result
                .try_get::<DateTime<chrono::Utc>, &str>("current_timestamp")
                .is_ok(),
            Err(_) => false,
        }
    }
}
