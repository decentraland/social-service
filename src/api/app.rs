use actix_web::body::MessageBody;
use actix_web::dev::{Server, ServiceFactory};
use actix_web::middleware;
use actix_web::{web::Data, App, HttpServer};
use dcl_http_prom_metrics::HttpMetricsCollector;
use tracing_actix_web::TracingLogger;

use crate::components::app::AppComponents;
use crate::components::configuration::Config;
use crate::components::tracing::init_telemetry;

use super::middlewares::check_auth::CheckAuthToken;
use super::middlewares::metrics_token::CheckMetricsToken;
use super::routes::health::handlers::health;
use super::routes::health::handlers::live;
use super::routes::synapse::handlers::{login, version};
use super::routes::synapse::room_events::room_event_handler;
use super::routes::v1::friendships::get::get_user_friends;
use super::routes::v1::friendships::mutuals::get_mutual_friends;

#[derive(Clone)]
pub struct AppOptions {
    pub auth_routes: Option<Vec<String>>,
}

pub fn run_service(data: Data<AppComponents>) -> Result<Server, std::io::Error> {
    init_telemetry();
    let server_port = data.config.server.port;

    let http_metrics_collector =
        Data::new(dcl_http_prom_metrics::HttpMetricsCollectorBuilder::default().build());

    let server = HttpServer::new(move || get_app_router(&data, &http_metrics_collector))
        .bind(("0.0.0.0", server_port))?
        .run();

    Ok(server)
}

pub async fn get_app_data(custom_config: Option<Config>) -> Data<AppComponents> {
    let app_data = AppComponents::new(custom_config).await;
    Data::new(app_data)
}

const ROUTES_NEED_AUTH_TOKEN: [&str; 3] = [
    "/v1/friendships/{userId}",
    "/v1/friendships/{userId}/mutuals",
    "/_matrix/client/r0/rooms/{room_id}/state/org.decentraland.friendship",
]; // should fill this array to protect routes

pub fn get_app_router(
    data: &Data<AppComponents>,
    http_metrics_collector: &Data<HttpMetricsCollector>,
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
        .app_data(http_metrics_collector.clone())
        .wrap(CheckAuthToken::new(protected_routes))
        .wrap(dcl_http_prom_metrics::metrics())
        .wrap(CheckMetricsToken::new(
            data.config.wkc_metrics_bearer_token.clone(),
        ))
        .wrap(middleware::NormalizePath::trim())
        .wrap(TracingLogger::default())
        .service(live)
        .service(health)
        .service(version)
        .service(get_user_friends)
        .service(get_mutual_friends)
        .service(login)
        .service(room_event_handler)
}
