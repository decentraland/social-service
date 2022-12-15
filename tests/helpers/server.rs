use actix_web::{body::MessageBody, dev::ServiceFactory, web::Data, App};
use social_service::{
    components::{
        app::{AppComponents, CustomComponents},
        configuration::{Config, Database},
    },
    get_app_router,
};
use sqlx::{Connection, Executor, PgConnection, PgPool};

pub fn get_configuration() -> Config {
    let mut config = Config::new().expect("Couldn't read the configuration file");
    config.db.name = uuid::Uuid::new_v4().to_string();
    config
}

pub async fn get_app(
    config: Config,
    custom_components: Option<CustomComponents>,
) -> App<
    impl ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    create_test_db(&config.db).await;
    let app_components = AppComponents::new(Some(config), custom_components).await;
    let app_data = Data::new(app_components);
    let app = get_app_router(&app_data);

    app
}

/// We need this to avoid conccurency issues in Tests
pub async fn create_test_db(db_config: &Database) -> PgPool {
    let url = format!(
        "postgres://{}:{}@{}",
        db_config.user, db_config.password, db_config.host
    );

    let mut connection = PgConnection::connect(url.as_str())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, db_config.name).as_str())
        .await
        .expect("Failed to create database");

    let url = format!(
        "postgres://{}:{}@{}/{}",
        db_config.user, db_config.password, db_config.host, db_config.name
    );

    let pool = PgPool::connect(&url)
        .await
        .expect("Failed to connect to Postgress.");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate DB");

    pool
}
