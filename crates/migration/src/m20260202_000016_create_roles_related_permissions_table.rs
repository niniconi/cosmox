use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RolesRelatedPermissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RolesRelatedPermissions::Rrpid)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(RolesRelatedPermissions::Rid)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RolesRelatedPermissions::Pid)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RolesRelatedPermissions::Builtin)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-rrp-role_id")
                            .from(RolesRelatedPermissions::Table, RolesRelatedPermissions::Rid)
                            .to(Roles::Table, Roles::Rid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-rrp-permission_id")
                            .from(RolesRelatedPermissions::Table, RolesRelatedPermissions::Pid)
                            .to(Permissions::Table, Permissions::Pid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(RolesRelatedPermissions::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum RolesRelatedPermissions {
    Table,
    Rrpid,
    Rid,
    Pid,
    Builtin,
}

#[derive(DeriveIden)]
enum Roles {
    Table,
    Rid,
}

#[derive(DeriveIden)]
enum Permissions {
    Table,
    Pid,
}
