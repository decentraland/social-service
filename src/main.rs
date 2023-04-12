use std::{io, sync::Arc};

use social_service::{
    api::app::{get_app_data, run_service},
    ws::app::{run_ws_transport, ConfigRpcServer, SocialContext},
};
use tokio::join;

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Run HTTP Server
    let app_data = get_app_data(None).await;
    let server = run_service(app_data.clone()).unwrap();

    // Run WebSocket transport
    let ctx = SocialContext {
        synapse: Arc::clone(&app_data.synapse),
        db: Arc::clone(&app_data.db),
        users_cache: Arc::clone(&app_data.users_cache),
        config: ConfigRpcServer {
            rpc_server: app_data.config.rpc_server.clone(),
        },
    };

    let (rpc_server_handle, http_server_handle) = run_ws_transport(ctx).await;

    // Wait for all tasks to complete
    // TODO: Handle gracefully shootdown
    let _ = join!(server, rpc_server_handle, http_server_handle);

    Ok(())
}
