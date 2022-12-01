use sea_orm_migration::prelude::*;

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
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(FriendshipHistory::FriendshipId)
                            .not_null()
                            .integer(),
                    )
                    .col(ColumnDef::new(FriendshipHistory::Event).not_null().string())
                    .col(
                        ColumnDef::new(FriendshipHistory::ActingUser)
                            .not_null()
                            .string(),
                    )
                    .col(ColumnDef::new(FriendshipHistory::Metadata).null().json())
                    .col(
                        ColumnDef::new(FriendshipHistory::Timestamp)
                            .not_null()
                            .date_time()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()), // adds the default option manually due to no option to set CURRENT_TIMESTAMP as default
                    )
                    .to_owned(),
            )
            .await
    }

    // Define how to rollback this migration: Drop the table.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
    Metadata,
    Timestamp,
}
