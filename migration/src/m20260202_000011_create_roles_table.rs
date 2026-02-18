use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(Roles::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(Roles::Rid)
              .big_unsigned()
              .not_null()
              .auto_increment()
              .primary_key(),
          )
          .col(ColumnDef::new(Roles::Name).string().not_null().unique_key())
          .col(ColumnDef::new(Roles::Description).string().null())
          .col(ColumnDef::new(Roles::Builtin).boolean().not_null())
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(Roles::Table).to_owned())
      .await
  }
}

#[derive(DeriveIden)]
enum Roles {
  Table,
  Rid,
  Name,
  Description,
  Builtin,
}
