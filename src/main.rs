use std::io;

use crate::routes::health::live::live;
use actix_web::{web::Data, App, HttpServer};
use components::app::AppComponents;
use configuration::Config;
use log;

mod components;
mod configuration;
mod metrics;
mod routes;

#[actix_web::main]
async fn main() -> io::Result<()> {
    // logger initialization change implementation depending on need
    env_logger::init();

    let data = Data::new(AppComponents::default());

    let configuration = Config::new().unwrap();

    log::info!("System is running on port {}", configuration.server.port);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(metrics::initialize_metrics())
            .service(live)
    })
    .bind((configuration.server.host, configuration.server.port))?
    .run()
    .await
}
