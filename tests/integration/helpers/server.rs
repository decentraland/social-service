use actix_web::rt::task::JoinHandle;

use social_service::run_service;

pub async fn start_server() -> JoinHandle<Result<(), std::io::Error>> {
    let server = run_service(None).await;

    if let Ok(server) = server {
        actix_web::rt::spawn(server)
    } else {
        panic!("Couldn't run the server");
    }
}
