use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(ResourcesRelatedTags::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(ResourcesRelatedTags::Rrtid)
              .big_unsigned()
              .not_null()
              .auto_increment()
              .primary_key(),
          )
          .col(
            ColumnDef::new(ResourcesRelatedTags::Rid)
              .big_unsigned()
              .not_null(),
          )
          .col(
            ColumnDef::new(ResourcesRelatedTags::Tid)
              .big_unsigned()
              .not_null(),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk-rrt-resource_id")
              .from(ResourcesRelatedTags::Table, ResourcesRelatedTags::Rid)
              .to(Resources::Table, Resources::Rid)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk-rrt-tag_id")
              .from(ResourcesRelatedTags::Table, ResourcesRelatedTags::Tid)
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
      .drop_table(Table::drop().table(ResourcesRelatedTags::Table).to_owned())
      .await
  }
}

#[derive(DeriveIden)]
enum ResourcesRelatedTags {
  Table,
  Rrtid,
  Rid,
  Tid,
}

#[derive(DeriveIden)]
enum Resources {
  Table,
  Rid,
}

#[derive(DeriveIden)]
enum Tags {
  Table,
  Tid,
}
