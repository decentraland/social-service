use std::io;

use actix_web::{get, web::Data, App, HttpResponse, HttpServer};
use components::AppComponents;
use configuration::Config;
use log;

mod components;
mod configuration;
mod metrics;

#[get("/ping")]
async fn ping(_app_data: Data<AppComponents>) -> HttpResponse {
    HttpResponse::Ok().json("pong")
}

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
            .wrap(metrics::initializeMetrics())
            .service(ping)
    })
    .bind((configuration.server.host, configuration.server.port))?
    .run()
    .await
}
