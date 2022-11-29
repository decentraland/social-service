use sea_orm_migration::prelude::*;

mod m20221125_000001_friendships_table;
mod m20221125_000002_friendship_history_table;
mod m20221125_000003_friendship_history_events;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20221125_000001_friendships_table::Migration),
            Box::new(m20221125_000002_friendship_history_table::Migration),
            Box::new(m20221125_000003_friendship_history_events::Migration),
        ]
    }
}