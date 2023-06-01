use std::{io, sync::Arc};

use social_service::{
    api::app::{get_app_data, run_service},
    ws::app::{init_ws_components, run_ws_transport, ConfigRpcServer, SocialContext},
};
use tokio::join;

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Get AppComponents
    let app_data = get_app_data(None).await;
    // Run HTTP Server
    let server = run_service(app_data.clone()).unwrap();

    // Get components WS specific
    let ws_components = init_ws_components(app_data.config.clone()).await;

    // Create Context to run RPC WebSocket transport
    let ctx = SocialContext {
        synapse: app_data.synapse.clone(),
        db: app_data.db.clone(),
        users_cache: Arc::clone(&app_data.users_cache),
        config: ConfigRpcServer {
            rpc_server: app_data.config.rpc_server.clone(),
            wkc_metrics_bearer_token: app_data.config.wkc_metrics_bearer_token.clone(),
        },
        redis_publisher: ws_components.redis_publisher.clone(),
        redis_subscriber: ws_components.redis_subscriber.clone(),
        friendships_events_generators: ws_components.friendships_events_generators.clone(),
        transport_context: ws_components.transport_context.clone(),
        friends_stream_page_size: app_data.config.friends_stream_page_size,
        metrics: ws_components.metrics,
    };
    // Run RPC Websocket Transport
    let (rpc_server_handle, http_server_handle) = run_ws_transport(ctx).await;

    // Wait for all tasks to complete
    // TODO: Handle gracefully shootdown
    let _ = join!(server, rpc_server_handle, http_server_handle);

    Ok(())
}
