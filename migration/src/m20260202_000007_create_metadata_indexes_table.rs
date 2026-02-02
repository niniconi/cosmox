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
              .auto_increment()
              .primary_key(),
          )
          .col(ColumnDef::new(MetadataIndexes::Path).string().not_null())
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
