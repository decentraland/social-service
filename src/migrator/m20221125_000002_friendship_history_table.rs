use sea_orm_migration::prelude::*;

use super::m20221125_000001_friendships_table::Friendships;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20221125_000002_friendship_history_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration: Create the table.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FriendshipHistory::Table)
                    .col(
                        ColumnDef::new(FriendshipHistory::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(FriendshipHistory::FriendshipId)
                            .not_null()
                            .integer(),
                    )
                    .col(
                        ColumnDef::new(FriendshipHistory::Event)
                            .not_null()
                            .integer(),
                    )
                    .col(
                        ColumnDef::new(FriendshipHistory::ActingUser)
                            .not_null()
                            .string(),
                    )
                    .col(
                        ColumnDef::new(FriendshipHistory::Timestamp)
                            .not_null()
                            .date_time()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()), // adds the default option manually due to no option to set CURRENT_TIMESTAMP as default
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                sea_query::ForeignKey::create()
                    .name("fk_frienship_history_friendships")
                    .from_tbl(FriendshipHistory::Table)
                    .from_col(FriendshipHistory::FriendshipId)
                    .to_tbl(Friendships::Table)
                    .to_col(Friendships::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await
    }

    // Define how to rollback this migration: Drop the table.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_frienship_history_friendships")
                    .table(FriendshipHistory::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(FriendshipHistory::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum FriendshipHistory {
    Table,
    Id,
    FriendshipId,
    Event,
    ActingUser,
    Timestamp,
}
