pub mod api;
pub mod components;
pub mod db;
pub mod domain;
pub mod entities;
pub mod synapse;
pub mod ws;
pub mod friendships {
    include!(concat!(
        env!("OUT_DIR"),
        "/decentraland.social.friendships.rs"
    ));
}
pub mod notifications {
    include!(concat!(
        env!("OUT_DIR"),
        "/decentraland.social.notifications.rs"
    ));
}

fn generate_uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}
