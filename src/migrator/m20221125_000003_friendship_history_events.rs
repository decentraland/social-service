use sea_orm_migration::prelude::*;
use sea_query::Query;

use super::m20221125_000002_friendship_history_table::FriendshipHistory;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20221125_000003_friendship_history_events"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FriendshipHistoryEvents::Table)
                    .col(
                        ColumnDef::new(FriendshipHistoryEvents::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(FriendshipHistoryEvents::Name)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                sea_query::ForeignKey::create()
                    .name("fk_frienship_history_history_events")
                    .from_tbl(FriendshipHistory::Table)
                    .from_col(FriendshipHistory::Event)
                    .to_tbl(FriendshipHistoryEvents::Table)
                    .to_col(FriendshipHistoryEvents::Id)
                    .to_owned(),
            )
            .await?;
        // Fill table with all the current events that we want
        // If we need to add more events we should create another migration
        // `make migration name=new_events`
        let events_query = Query::insert()
            .into_table(FriendshipHistoryEvents::Table)
            .columns([FriendshipHistoryEvents::Name])
            .values_panic(["requested".into()])
            .values_panic(["accepted".into()])
            .values_panic(["rejected".into()])
            .values_panic(["removed".into()])
            .values_panic(["block".into()])
            .values_panic(["unblock".into()])
            .values_panic(["message_on_request".into()])
            .to_owned();
        manager.exec_stmt(events_query).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_frienship_history_history_events")
                    .table(FriendshipHistory::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(FriendshipHistoryEvents::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
pub enum FriendshipHistoryEvents {
    Table,
    Id,
    Name,
}