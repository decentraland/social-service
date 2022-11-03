use std::io;

use social_service::{get_app_data, run_service};

#[actix_web::main]
async fn main() -> io::Result<()> {
    // logger initialization change implementation depending on need

    let app_data = get_app_data().await;

    let server = run_service(None, app_data);
    if let Ok(server) = server {
        server.await?;
    }

    Ok(())
}
