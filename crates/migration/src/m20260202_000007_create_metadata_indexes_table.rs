use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MetadataIndexes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MetadataIndexes::Mid)
                            .big_unsigned()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MetadataIndexes::Path).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-metadata_indexes-resource")
                            .from(MetadataIndexes::Table, MetadataIndexes::Mid)
                            .to(Resources::Table, Resources::Rid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MetadataIndexes::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MetadataIndexes {
    Table,
    Mid,
    Path,
}

#[derive(DeriveIden)]
enum Resources {
    Table,
    Rid,
}
