// TODO: delete file
use dcl_rpc::{client::RpcClient, transports::web_socket::WebSocketTransport};
use tokio_tungstenite::tungstenite;

use tungstenite::client::IntoClientRequest;

use social_service::FriendshipsServiceClient;

use social_service::RPCServiceClient;

#[tokio::main]
async fn main() {
    // Create WebSocket client request
    let ws_url = "ws://127.0.0.1:3030/ws";
    let mut req = match ws_url.into_client_request() {
        Ok(req) => req,
        Err(_) => panic!("Failed to create WebSocket request."),
    };

    // Set Authorization header
    let auth_token = "123";
    let auth_header = match auth_token.parse() {
        Ok(header) => header,
        Err(_) => panic!("Failed to parse Authorization header."),
    };
    req.headers_mut().insert("Authorization", auth_header);

    // Connect to WebSocket server
    let (ws_client, _) = match tokio_tungstenite::connect_async(req).await {
        Ok(ws) => ws,
        Err(_) => panic!("Failed to connect to the WebSocket upgraded connection."),
    };

    // Create RPC client with WebSocket transport
    let ws_transport = WebSocketTransport::new(ws_client);
    let mut rpc_client = match RpcClient::new(ws_transport).await {
        Ok(client) => client,
        Err(_) => panic!("Failed to create RPC client."),
    };

    // Create client port and load FriendshipService module
    let client_port = match rpc_client.create_port("TEST_port").await {
        Ok(port) => port,
        Err(_) => panic!("Failed to create RPC client port."),
    };
    let friendship_service_client = match client_port
        .load_module::<FriendshipsServiceClient<WebSocketTransport>>("FriendshipsService")
        .await
    {
        Ok(client) => client,
        Err(_) => panic!("Failed to load FriendshipService module."),
    };

    // Call RPC method to get friends
    let _response = friendship_service_client.get_friends().await;
}
