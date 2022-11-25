echo "use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "\"$1\""
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration: Create the table.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Model::Table)
                    //.col() -> Columns should be defined here
                    .to_owned(),
            )
            .await
    }

    // Define how to rollback this migration: Drop the table.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Model::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
// THIS SHOULD BE REPLACED BY YOUR TABLE
pub enum Model {
    Table,
}
" > ./src/migrator/$1.rs

FILES=$(ls src/migrator | grep -E ".*_([0-9]{6})_.*" | sed 's/.rs//' | sed 's/\(.*\)/mod \1;/')
BOXES=$(ls src/migrator | grep -E ".*_([0-9]{6})_.*" | sed 's/.rs//' | sed 's/\(.*\)/Box::new(\1::Migration),/')

echo "use sea_orm_migration::prelude::*;

${FILES}

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
           ${BOXES}
        ]
    }
}
" > ./src/migrator/mod.rs