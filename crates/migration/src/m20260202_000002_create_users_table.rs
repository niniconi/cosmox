use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Uid)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Users::Password).string().not_null())
                    .col(
                        ColumnDef::new(Users::Username)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Users::Avatar).big_unsigned().null())
                    .col(ColumnDef::new(Users::Nickname).string().null())
                    .col(ColumnDef::new(Users::CreateDatetime).date_time().not_null())
                    .col(
                        ColumnDef::new(Users::LastUpdateDatetime)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Users::Email).string().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-users-avatar")
                            .from(Users::Table, Users::Avatar)
                            .to(PathMappings::Table, PathMappings::Pmid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Uid,
    Password,
    Username,
    Avatar,
    Nickname,
    CreateDatetime,
    LastUpdateDatetime,
    Email,
}

#[derive(DeriveIden)]
enum PathMappings {
    Table,
    Pmid,
}
