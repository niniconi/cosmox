use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(LibrarysRelatedTags::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(LibrarysRelatedTags::Lrtid)
              .big_unsigned()
              .not_null()
              .auto_increment()
              .primary_key(),
          )
          .col(
            ColumnDef::new(LibrarysRelatedTags::Lid)
              .big_unsigned()
              .not_null(),
          )
          .col(
            ColumnDef::new(LibrarysRelatedTags::Tid)
              .big_unsigned()
              .not_null(),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk-lrt-library_id")
              .from(LibrarysRelatedTags::Table, LibrarysRelatedTags::Lid)
              .to(Librarys::Table, Librarys::Lid)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk-lrt-tag_id")
              .from(LibrarysRelatedTags::Table, LibrarysRelatedTags::Tid)
              .to(Tags::Table, Tags::Tid)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(LibrarysRelatedTags::Table).to_owned())
      .await
  }
}

#[derive(DeriveIden)]
enum LibrarysRelatedTags {
  Table,
  Lrtid,
  Lid,
  Tid,
}

#[derive(DeriveIden)]
enum Librarys {
  Table,
  Lid,
}

#[derive(DeriveIden)]
enum Tags {
  Table,
  Tid,
}
