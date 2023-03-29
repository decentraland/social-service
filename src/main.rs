use std::io;

use social_service::{
    api::app::{get_app_data, run_service},
    ws::app::run_ws_transport,
};
use tokio::join;

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Get application data
    let app_data = get_app_data(None).await;

    // Run the http service
    // We want the service to panic if starting the server fails
    let server = run_service(app_data).unwrap();

    // Run WebSocket transport
    let (rpc_server_handle, http_server_handle) = run_ws_transport().await;

    // Wait for all tasks to complete
    let _ = join!(server, rpc_server_handle, http_server_handle);

    Ok(())
}
