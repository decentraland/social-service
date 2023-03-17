use dcl_rpc::{
    server::{RpcServer, RpcServerPort},
    transports::web_socket::{WebSocketServer, WebSocketTransport},
};

// TODO!: suspicious imports ?) 0.0
use crate::{ws::service::friendships_service, FriendshipsServiceRegistration, User};

pub async fn run_ws_transport() {
    let ws_server = WebSocketServer::new("127.0.0.1:8085");

    let mut connection_listener = ws_server.listen().await.unwrap();

    let ctx = MyExampleContext {
        hardcoded_database: create_db(),
    };

    let mut server = RpcServer::create(ctx);
    server.set_handler(|port: &mut RpcServerPort<MyExampleContext>| {
        println!("Registering Rust Social WS Server");
        FriendshipsServiceRegistration::register_service(
            port,
            friendships_service::MyFriendshipsService {},
        );
    });

    // The WebSocket Server listens for incoming connections, when a connection is established,
    // it creates a new WebSocketTransport with that connection and attaches it to the server event sender.
    // The loop continues to listen for incoming connections and attach transports until it is stopped.
    let server_events_sender = server.get_server_events_sender();
    tokio::spawn(async move {
        while let Some(Ok(connection)) = connection_listener.recv().await {
            let transport = WebSocketTransport::new(connection);
            match server_events_sender.send_attach_transport(transport) {
                Ok(_) => {
                    println!("> RpcServer > Transport attached successfully.");
                }
                Err(_) => {
                    println!("> RpcServer > Unable to attach transport.");
                    panic!()
                }
            }
        }
    });

    server.run().await;
}

fn create_db() -> Vec<User> {
    let user_1 = User {
        address: "0x111".to_string(),
    };

    vec![user_1]
}

pub struct MyExampleContext {
    pub hardcoded_database: Vec<User>,
}
