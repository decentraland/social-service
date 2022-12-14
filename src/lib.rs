pub mod components;
pub mod entities;
mod metrics;
pub mod middlewares;
pub mod routes;
mod utils;

use actix_web::body::MessageBody;
use actix_web::dev::{Server, ServiceFactory};
use actix_web::web;
use actix_web::{web::Data, App, HttpServer};
use components::app::{new_app, AppComponents};
use components::health::{Health, HealthComponent};
use components::synapse::{Synapse, SynapseComponent};
use tracing_actix_web::TracingLogger;

use components::{configuration::Config, tracing::init_telemetry};
use metrics::initialize_metrics;
use middlewares::metrics_token::CheckMetricsToken;
use routes::{
    health::handlers::{health, live, startup},
    synapse::handlers::version,
};

pub type AppData<H, S> = Data<AppComponents<H, S>>;

pub fn run_service(data: AppData<Health, Synapse>) -> Result<Server, std::io::Error> {
    init_telemetry();

    log::debug!("App Config: {:?}", data.config);

    let server_host = data.config.server.host.clone();
    let server_port = data.config.server.port;

    let server = HttpServer::new(move || get_app_router(&data))
        .bind((server_host, server_port))?
        .run();

    Ok(server)
}

pub async fn get_app_data(custom_config: Option<Config>) -> AppData<Health, Synapse> {
    let app = new_app(custom_config).await;
    Data::new(app)
}

pub fn get_app_router<H: HealthComponent + 'static, S: SynapseComponent + 'static>(
    data: &AppData<H, S>,
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
        .route("/_matrix/client/versions", web::get().to(version::<H, S>))
        .route("/health/live", web::get().to(live))
        .route("/health/ready", web::get().to(health::<H, S>))
        .route("/health/startup", web::get().to(startup::<H, S>))
}

fn generate_uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}
