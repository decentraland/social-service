use crate::metrics::initialize_metrics;
use crate::routes::health::live::live;
use crate::{components::tracing::init_telemetry, routes::health::health::health};

use actix_web::dev::Server;
use actix_web::{web::Data, App, HttpServer};
use components::app::AppComponents;
use configuration::Config;
use tracing_actix_web::TracingLogger;

pub mod components;
mod configuration;
mod metrics;
pub mod routes;

pub async fn run_service(custom_config: Option<Config>) -> Result<Server, std::io::Error> {
    // logger initialization change implementation depending on need
    env_logger::init();

    let config =
        custom_config.unwrap_or(Config::new().expect("Couldn't read the configuration file"));

    init_telemetry();

    let app_data = AppComponents::new().await;

    let data = Data::new(app_data);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(initialize_metrics())
            .wrap(TracingLogger::default())
            .service(live)
            .service(health)
    })
    .bind((config.server.host, config.server.port))?
    .run();

    Ok(server)
}
