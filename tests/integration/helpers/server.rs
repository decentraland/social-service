use actix_web::rt::task::JoinHandle;

use social_service::{configuration::Config, run_service};

pub fn get_configuration() -> Config {
    Config::new().expect("Couldn't read the configuration file")
}

pub async fn start_server(config: Config) -> JoinHandle<Result<(), std::io::Error>> {
    let server = run_service(Some(config)).await;

    if let Ok(server) = server {
        actix_web::rt::spawn(server)
    } else {
        panic!("Couldn't run the server");
    }
}
