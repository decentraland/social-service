use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20221125_000001_friendships_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration: Create the table.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Friendships::Table)
                    .col(
                        ColumnDef::new(Friendships::Id)
                            .uuid()
                            .primary_key()
                            .extra("DEFAULT uuid_generate_v4()".to_string()),
                    )
                    .col(ColumnDef::new(Friendships::Address1).string().not_null())
                    .col(ColumnDef::new(Friendships::Address2).string().not_null())
                    .col(
                        ColumnDef::new(Friendships::IsActive)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                sea_query::Index::create()
                    .name("unique_friendship_index")
                    .unique()
                    .table(Friendships::Table)
                    .col(Friendships::Address1)
                    .col(Friendships::Address2)
                    .to_owned(),
            )
            .await
    }

    // Define how to rollback this migration: Drop the table.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("unique_friendship_index").to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Friendships::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Friendships {
    Table,
    Id,
    Address1,
    Address2,
    IsActive,
}
