use std::{io, sync::Arc};

use social_service::{
    api::app::{get_app_data, run_service},
    ws::app::{run_ws_transport, ConfigRpcServer, SocialContext},
};
use tokio::join;

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Get AppComponents
    let app_data = get_app_data(None).await;
    // Run HTTP Server
    let server = run_service(app_data.clone()).unwrap();

    // Create Context to run RPC WebSocket transport
    let ctx = SocialContext {
        synapse: app_data.synapse.clone(),
        db: app_data.db.clone(),
        users_cache: Arc::clone(&app_data.users_cache),
        config: ConfigRpcServer {
            rpc_server: app_data.config.rpc_server.clone(),
        },
    };
    // Run RPC Websocket Transport
    let (rpc_server_handle, http_server_handle) = run_ws_transport(ctx).await;

    // Wait for all tasks to complete
    // TODO: Handle gracefully shootdown
    let _ = join!(server, rpc_server_handle, http_server_handle);

    Ok(())
}
