use actix_web::{body::MessageBody, dev::ServiceFactory, App};
use social_service::{configuration::Config, get_app_data, get_app_router};

pub fn get_configuration() -> Config {
    let mut config = Config::new().expect("Couldn't read the configuration file");
    config.server.port = 0;
    config
}

pub async fn get_app(
    config: Config,
) -> App<
    impl ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let app_data = get_app_data(Some(config)).await;
    let app = get_app_router(&app_data);

    app
}
