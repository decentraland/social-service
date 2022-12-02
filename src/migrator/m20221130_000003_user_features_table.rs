use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20221130_000003_user_features_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserFeatures::Table)
                    .col(ColumnDef::new(UserFeatures::User).string().not_null())
                    .col(
                        ColumnDef::new(UserFeatures::FeatureName)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserFeatures::FeatureValue)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserFeatures::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum UserFeatures {
    Table,
    User,
    FeatureName,
    FeatureValue,
}
