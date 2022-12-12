pub mod components;
pub mod entities;
mod metrics;
pub mod middlewares;
pub mod routes;
mod utils;

use actix_web::body::MessageBody;
use actix_web::dev::{Server, ServiceFactory};
use actix_web::{web::Data, App as ActixApp, HttpServer};
use components::app::{App, AppComponents};
use tracing_actix_web::TracingLogger;

use components::{configuration::Config, tracing::init_telemetry};
use metrics::initialize_metrics;
use middlewares::metrics_token::CheckMetricsToken;
use routes::{
    health::handlers::{health, live},
    synapse::handlers::version,
};

pub type AppData = Data<dyn AppComponents + Send + Sync>;

pub fn run_service(data: AppData) -> Result<Server, std::io::Error> {
    init_telemetry();

    let app_config = data.get_config();

    log::debug!("App Config: {:?}", app_config);

    let server_host = app_config.server.host.clone();
    let server_port = app_config.server.port;

    let server = HttpServer::new(move || get_app_router(&data))
        .bind((server_host, server_port))?
        .run();

    Ok(server)
}

pub async fn get_app_data(custom_config: Option<Config>) -> AppData {
    let app_data = App::new(custom_config).await;
    Data::from(app_data)
}

pub fn get_app_router(
    data: &AppData,
) -> ActixApp<
    impl ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let app_config = data.get_config();
    ActixApp::new()
        .app_data(data.clone())
        .wrap(TracingLogger::default())
        .wrap(initialize_metrics(app_config.env.clone()))
        .wrap(CheckMetricsToken::new(
            app_config.wkc_metrics_bearer_token.clone(),
        ))
        .service(live)
        .service(health)
        .service(version)
}

fn generate_uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}
