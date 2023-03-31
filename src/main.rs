use std::io;

use social_service::{
    api::app::{get_app_data, run_service},
    ws::app::run_ws_transport,
};
use tokio::join;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let app_data = get_app_data(None).await;
    let server = run_service(app_data.clone()).unwrap();

    let app_components = app_data.into_inner();
    // Run WebSocket transport
    let rpc_server_handle = run_ws_transport(app_components).await;

    // Wait for all tasks to complete
    let _ = join!(server, rpc_server_handle);

    Ok(())
}
