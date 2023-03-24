use dcl_rpc::{
    server::{RpcServer, RpcServerPort},
    transports::web_socket::{WebSocketServer, WebSocketTransport},
};

// TODO!: suspicious imports ?) 0.0
use crate::{
    components::{
        configuration::{Config, Database},
        database::{DatabaseComponent, DatabaseComponentImplementation},
    },
    ws::service::friendships_service,
    FriendshipsServiceRegistration,
};

pub async fn run_ws_transport() {
    let ws_server = WebSocketServer::new("127.0.0.1:8085");

    let mut connection_listener = ws_server.listen().await.unwrap();

    let config = Config::new().expect("Couldn't read the configuration");

    let ctx = MyContext {
        db: init_db_component(&config.db).await,
    };

    let mut server = RpcServer::create(ctx);
    server.set_handler(|port: &mut RpcServerPort<MyContext>| {
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

async fn init_db_component(db_config: &Database) -> DatabaseComponent {
    let mut db = DatabaseComponent::new(db_config);
    if let Err(err) = db.run().await {
        log::debug!("Error on running the DB: {:?}", err);
        panic!("Unable to run the DB")
    }
    db
}

pub struct MyContext {
    pub db: DatabaseComponent,
}
