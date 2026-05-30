use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Types::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Types::Tid)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Types::ScanMode).string().null())
                    .col(
                        ColumnDef::new(Types::Label)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Types::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Types {
    Table,
    Tid,
    ScanMode,
    Label,
}
