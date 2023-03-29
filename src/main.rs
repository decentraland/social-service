use std::io;

use social_service::{
    api::app::{get_app_data, run_service},
    ws::app::upgrade_to_ws,
};
use tokio::join;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let app_data = get_app_data(None).await;

    let server = run_service(app_data).unwrap();

    let (rpc_server_handle, http_server_handle) = upgrade_to_ws().await;

    let _ = join!(server, rpc_server_handle, http_server_handle);

    Ok(())
}
