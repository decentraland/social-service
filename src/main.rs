use std::io;

use crate::metrics::initialize_metrics;
use crate::routes::health::live::live;
use crate::{components::tracing::init_telemetry, routes::health::health::health};
use actix_web::{web::Data, App, HttpServer};
use components::app::AppComponents;
use configuration::Config;
use log;
use tracing_actix_web::TracingLogger;

mod components;
mod configuration;
mod metrics;
mod routes;

#[actix_web::main]
async fn main() -> io::Result<()> {
    // logger initialization change implementation depending on need
    env_logger::init();

    init_telemetry();

    let app_data = AppComponents::new().await;
    let data = Data::new(app_data);
    let configuration = Config::new().unwrap();

    log::info!("System is running on port {}", configuration.server.port);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(initialize_metrics())
            .wrap(TracingLogger::default())
            .service(live)
            .service(health)
    })
    .bind((configuration.server.host, configuration.server.port))?
    .run()
    .await
}
