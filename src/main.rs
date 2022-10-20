use std::io;

use social_service::run_service;

mod components;
mod configuration;
mod metrics;
mod routes;

#[actix_web::main]
async fn main() -> io::Result<()> {
    // logger initialization change implementation depending on need

    let server = run_service(None);
    server.await?.await;

    Ok(())
}
