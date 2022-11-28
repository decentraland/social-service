pub mod components;
pub mod entities;
mod metrics;
pub mod middlewares;
mod migrator;
pub mod routes;

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
