use actix_web::{body::MessageBody, dev::ServiceFactory, web::Data, App};
use social_service::{
    components::{
        app::AppComponents,
        configuration::{Config, Database},
        synapse::{WhoAmIResponse, WHO_AM_I_URI},
    },
    get_app_router,
};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use wiremock::{
    matchers::{method, path},
    MockServer, ResponseTemplate, Mock,
};

pub async fn get_configuration() -> Config {
    let mut config = Config::new().expect("Couldn't read the configuration file");
    config.db.name = uuid::Uuid::new_v4().to_string();
    create_test_db(&config.db).await;
    config
}

pub async fn get_app(
    config: Config,
    components: Option<AppComponents>,
) -> App<
    impl ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let app_components = components.unwrap_or(AppComponents::new(Some(config)).await);
    let app_data = Data::new(app_components);
    get_app_router(&app_data)
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

pub async fn create_synapse_mock_server() -> MockServer {
    MockServer::start().await
}

pub async fn mock_server_expect_no_calls() -> MockServer {
    let server = create_synapse_mock_server().await;
    Mock::given(method("GET"))
        .and(path(WHO_AM_I_URI))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .named("No calls to who am I")
        .mount(&server)
        .await;

    server
}

/// Creates a synapse mocked server which respond with the given user ID to who am I endpoint.
pub async fn who_am_i_synapse_mock_server(user_id: String) -> MockServer {
    let server = create_synapse_mock_server().await;
    let response = WhoAmIResponse { user_id };
    Mock::given(method("GET"))
        .and(path(WHO_AM_I_URI))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    server
}