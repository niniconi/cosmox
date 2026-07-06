use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Libraries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Libraries::Lid)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Libraries::Name).string().null())
                    .col(ColumnDef::new(Libraries::Description).string().null())
                    .col(
                        ColumnDef::new(Libraries::CreateDatetime)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Libraries::LastUpdateDatetime)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Libraries::Type).big_unsigned().not_null())
                    .col(
                        ColumnDef::new(Libraries::CreateByUid)
                            .big_unsigned()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-librarys-type")
                            .from(Libraries::Table, Libraries::Type)
                            .to(Types::Table, Types::Tid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-librarys-user")
                            .from(Libraries::Table, Libraries::CreateByUid)
                            .to(Users::Table, Users::Uid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Libraries::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Libraries {
    Table,
    Lid,
    Name,
    Description,
    CreateDatetime,
    LastUpdateDatetime,
    Type,
    CreateByUid,
}

#[derive(DeriveIden)]
enum Types {
    Table,
    Tid,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Uid,
}
