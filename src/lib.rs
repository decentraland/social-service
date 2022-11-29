pub mod components;
pub mod entities;
mod metrics;
pub mod middlewares;
mod migrator;
pub mod routes;

use std::env;
use std::ffi::OsString;

use actix_web::body::MessageBody;
use actix_web::dev::{Server, ServiceFactory};
use actix_web::{web::Data, App, HttpServer};
use tracing_actix_web::TracingLogger;

use components::{app::AppComponents, configuration::Config, tracing::init_telemetry};
use metrics::initialize_metrics;
use middlewares::metrics_token::CheckMetricsToken;
use routes::{
    health::handlers::{health, live},
    synapse::handlers::version,
};

#[derive(Debug)]
pub enum MigrationHelper {
    UP,
    DOWN,
    UNKNOWN,
}

pub fn run_service(data: Data<AppComponents>) -> Result<Server, std::io::Error> {
    init_telemetry();

    log::debug!("App Config: {:?}", data.config);

    let server_host = data.config.server.host.clone();
    let server_port = data.config.server.port;

    let server = HttpServer::new(move || get_app_router(&data))
        .bind((server_host, server_port))?
        .run();

    Ok(server)
}

pub async fn get_app_data(custom_config: Option<Config>) -> Data<AppComponents> {
    let app_data = AppComponents::new(custom_config).await;
    Data::new(app_data)
}

pub fn get_app_router(
    data: &Data<AppComponents>,
) -> App<
    impl ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(data.clone())
        .wrap(TracingLogger::default())
        .wrap(initialize_metrics(data.config.env.clone()))
        .wrap(CheckMetricsToken::new(
            data.config.wkc_metrics_bearer_token.clone(),
        ))
        .service(live)
        .service(health)
        .service(version)
}

pub fn should_run_migration_helper() -> bool {
    let migration_helper = env::var_os("MIGRATE");

    if is_local() && migration_helper.is_some() {
        true
    } else {
        false
    }
}

pub fn is_local() -> bool {
    let local = env::var_os("LOCAL");
    if local.is_some() {
        if local.unwrap() == "true" {
            true
        } else {
            false
        }
    } else {
        false
    }
}

pub fn get_migration_helper() -> (MigrationHelper, Option<u32>) {
    let migration_helper = env::var_os("MIGRATE").unwrap_or(OsString::from(""));
    let migration_helper_count = env::var_os("COUNT").unwrap_or(OsString::from("0")); // How many migrations back or up
    let migration_helper_count = migration_helper_count
        .into_string()
        .unwrap()
        .parse::<u32>()
        .unwrap();

    let migration_helper_count = if migration_helper_count == 0 {
        None
    } else {
        Some(migration_helper_count)
    };

    if migration_helper == "up" {
        (MigrationHelper::UP, migration_helper_count)
    } else if migration_helper == "down" {
        (MigrationHelper::DOWN, migration_helper_count)
    } else {
        (MigrationHelper::UNKNOWN, migration_helper_count)
    }
}
