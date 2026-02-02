use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(UsersRelatedRoles::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(UsersRelatedRoles::Urrid)
              .big_unsigned()
              .not_null()
              .auto_increment()
              .primary_key(),
          )
          .col(
            ColumnDef::new(UsersRelatedRoles::Uid)
              .big_unsigned()
              .not_null(),
          )
          .col(
            ColumnDef::new(UsersRelatedRoles::Rid)
              .big_unsigned()
              .not_null(),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk-urr-user_id")
              .from(UsersRelatedRoles::Table, UsersRelatedRoles::Uid)
              .to(Users::Table, Users::Uid)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk-urr-role_id")
              .from(UsersRelatedRoles::Table, UsersRelatedRoles::Rid)
              .to(Roles::Table, Roles::Rid)
              .on_delete(ForeignKeyAction::Cascade)
              .on_update(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(UsersRelatedRoles::Table).to_owned())
      .await
  }
}

#[derive(DeriveIden)]
enum UsersRelatedRoles {
  Table,
  Urrid,
  Uid,
  Rid,
}

#[derive(DeriveIden)]
enum Users {
  Table,
  Uid,
}

#[derive(DeriveIden)]
enum Roles {
  Table,
  Rid,
}
