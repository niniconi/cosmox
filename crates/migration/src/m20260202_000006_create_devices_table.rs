use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Devices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Devices::Did)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Devices::UserAgent).string().null())
                    .col(ColumnDef::new(Devices::LoginByUid).big_unsigned().null())
                    .col(
                        ColumnDef::new(Devices::LastLoginDatetime)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Devices::LastLoginIp).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-devices-login_by_uid")
                            .from(Devices::Table, Devices::LoginByUid)
                            .to(Users::Table, Users::Uid)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Devices::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Devices {
    Table,
    Did,
    UserAgent,
    LoginByUid,
    LastLoginDatetime,
    LastLoginIp,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Uid,
}
