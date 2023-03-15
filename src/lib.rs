pub mod api;
pub mod components;
pub mod entities;
mod metrics;
pub mod middlewares;
mod utils;
pub mod ws;
// include!(concat!(env!("OUT_DIR"), "/decentraland.social.friendships.rs"));

fn generate_uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub struct MyExampleContext {
    // pub hardcoded_database: Vec<User>,
}
