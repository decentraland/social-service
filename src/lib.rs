use crate::metrics::initialize_metrics;
use crate::routes::health::live::live;
use crate::{
    components::tracing::init_telemetry,
    routes::{health::controllers::health, synapse::controllers::version},
};

use actix_web::dev::Server;
use actix_web::{web::Data, App, HttpServer};
use components::app::AppComponents;
use configuration::Config;
use tracing_actix_web::TracingLogger;

pub mod components;
pub mod configuration;
mod metrics;
pub mod routes;

pub fn run_service(data: Data<AppComponents>) -> Result<Server, std::io::Error> {
    // logger initialization change implementation depending on need
    env_logger::init();

    init_telemetry();

    let server_host = data.config.server.host.clone();
    let server_port = data.config.server.port;

    let server = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(initialize_metrics())
            .wrap(TracingLogger::default())
            .service(live)
            .service(health)
            .service(version)
    })
    .bind((server_host, server_port))?
    .run();

    Ok(server)
}

pub async fn get_app_data(custom_config: Option<Config>) -> Data<AppComponents> {
    let app_data = AppComponents::new(custom_config).await;
    Data::new(app_data)
}
