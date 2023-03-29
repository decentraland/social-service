use std::io;

use social_service::api::app::{get_app_data, run_service};

#[actix_web::main]
async fn main() -> io::Result<()> {
    let app_data = get_app_data(None).await;

    let server = run_service(app_data);
    if let Ok(server) = server {
        server.await?;
    }

    Ok(())
}
