use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(PathMappings::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(PathMappings::Pmid)
              .big_unsigned()
              .not_null()
              .auto_increment()
              .primary_key(),
          )
          .col(ColumnDef::new(PathMappings::Path).string().not_null())
          .col(ColumnDef::new(PathMappings::MimeType).string().not_null())
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(PathMappings::Table).to_owned())
      .await
  }
}

#[derive(DeriveIden)]
enum PathMappings {
  Table,
  Pmid,
  Path,
  MimeType,
}
