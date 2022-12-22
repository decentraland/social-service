pub mod components;
pub mod entities;
mod metrics;
pub mod middlewares;
pub mod routes;
mod utils;

use actix_web::body::MessageBody;
use actix_web::dev::{Server, ServiceFactory};
use actix_web::{web::Data, App, HttpServer};
use middlewares::check_auth::CheckAuthToken;
use tracing_actix_web::TracingLogger;

use components::{app::AppComponents, configuration::Config, tracing::init_telemetry};
use metrics::initialize_metrics;
use middlewares::metrics_token::CheckMetricsToken;
use routes::v1::friendships::get::get_user_friends;
use routes::{
    health::handlers::{health, live},
    synapse::handlers::{login, version},
};

#[derive(Clone)]
pub struct AppOptions {
    pub auth_routes: Option<Vec<String>>,
}

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
    let app_data = AppComponents::new(custom_config, None).await;
    Data::new(app_data)
}

const ROUTES_NEED_AUTH_TOKEN: [&str; 1] = ["/v1/friendships/{userId}"]; // should fill this array to protect routes

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
    let protected_routes = ROUTES_NEED_AUTH_TOKEN
        .iter()
        .map(|s| String::from(*s))
        .collect::<Vec<String>>();

    App::new()
        .app_data(data.clone())
        .wrap(TracingLogger::default())
        .wrap(initialize_metrics(data.config.env.clone()))
        .wrap(CheckMetricsToken::new(
            data.config.wkc_metrics_bearer_token.clone(),
        ))
        .wrap(CheckAuthToken::new(protected_routes))
        .service(live)
        .service(health)
        .service(version)
        .service(get_user_friends)
        .service(login)
}

fn generate_uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}
