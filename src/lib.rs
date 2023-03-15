pub mod api;
pub mod components;
pub mod entities;
mod metrics;
pub mod middlewares;
mod utils;

fn generate_uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}
