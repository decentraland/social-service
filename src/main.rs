use std::io;

use actix_web::{get, web::Data, App, HttpResponse, HttpServer};
use components::AppComponents;
use configuration::Config;

mod components;
mod configuration;

#[get("/items")]
async fn ping(_app_data: Data<AppComponents>) -> HttpResponse {
    HttpResponse::Ok().json("pong")
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let data = Data::new(AppComponents::default());
    let configuration = Config::new().unwrap();

    HttpServer::new(move || App::new().app_data(data.clone()).service(ping))
        .bind((configuration.server.host, configuration.server.port))?
        .run()
        .await
}
