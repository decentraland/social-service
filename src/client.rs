use dcl_rpc::{
    client::RpcClient,
    transports::web_socket::{WebSocketClient, WebSocketTransport},
};

use social_service::{AuthToken, FriendshipsServiceClient, RPCServiceClient};

#[tokio::main]
async fn main() {
    let client_connection = WebSocketClient::connect("ws://127.0.0.1:8085")
        .await
        .unwrap();

    let client_transport = WebSocketTransport::new(client_connection);
    let mut client = RpcClient::new(client_transport).await.unwrap();
    let port = client.create_port("echo").await.unwrap();

    let module = port
        .load_module::<FriendshipsServiceClient<WebSocketTransport>>("FriendshipsService")
        .await
        .unwrap();

    // Get All Friends message
    let mut friends_response = module
        .get_friends(AuthToken {
            synapse_token: "".to_string(),
        })
        .await;

    while let Some(friend) = friends_response.next().await {
        println!(
            "> Server Streams > Response > GetAllFriendsResponse {:?}",
            friend.address
        )
    }
}
