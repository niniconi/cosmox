use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(LibraryPaths::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(LibraryPaths::Lpid)
              .big_unsigned()
              .not_null()
              .auto_increment()
              .primary_key(),
          )
          .col(ColumnDef::new(LibraryPaths::Lid).big_unsigned().not_null())
          .col(ColumnDef::new(LibraryPaths::Path).string().not_null())
          .foreign_key(
            ForeignKey::create()
              .name("fk-library_paths-lid")
              .from(LibraryPaths::Table, LibraryPaths::Lid)
              .to(Librarys::Table, Librarys::Lid)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(LibraryPaths::Table).to_owned())
      .await
  }
}

#[derive(DeriveIden)]
enum LibraryPaths {
  Table,
  Lpid,
  Lid,
  Path,
}

#[derive(DeriveIden)]
enum Librarys {
  Table,
  Lid,
}
