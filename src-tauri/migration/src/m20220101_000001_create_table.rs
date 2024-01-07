use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .create_table(
                Table::create()
                    .table(Capture::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Capture::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Capture::Timestamp).date_time().not_null())
                    .col(ColumnDef::new(Capture::Path).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .drop_table(Table::drop().table(Capture::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Capture {
    Table,
    Id,
    Timestamp,
    Path,
}
