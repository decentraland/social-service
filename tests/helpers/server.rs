use actix_web::rt::task::JoinHandle;

use social_service::{configuration::Config, get_app_data, run_service};

pub fn get_configuration() -> Config {
    Config::new().expect("Couldn't read the configuration file")
}

pub async fn start_server(config: Config) -> JoinHandle<Result<(), std::io::Error>> {
    let app_data = get_app_data(Some(config)).await;
    let server = run_service(app_data);

    let server = server.unwrap_or_else(|_| panic!("Couldn't run the server"));
    actix_web::rt::spawn(server)
}
