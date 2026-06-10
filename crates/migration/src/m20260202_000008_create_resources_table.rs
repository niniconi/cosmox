use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Resources::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Resources::Rid)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Resources::Name).string().null())
                    .col(ColumnDef::new(Resources::Description).string().null())
                    .col(ColumnDef::new(Resources::Lid).big_unsigned().null())
                    .col(
                        ColumnDef::new(Resources::CreateDatetime)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Resources::LastUpdateDatetime)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Resources::Level)
                            .big_unsigned()
                            .not_null()
                            .comment("The purpose of this field is to record the nesting level of the current resource, starting the count from 0."),
                    )
                    .col(ColumnDef::new(Resources::Cover).big_unsigned().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-resources-lid")
                            .from(Resources::Table, Resources::Lid)
                            .to(Libraries::Table, Libraries::Lid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-resources-cover")
                            .from(Resources::Table, Resources::Cover)
                            .to(PathMappings::Table, PathMappings::Pmid)
                            .on_delete(ForeignKeyAction::NoAction)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Resources::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Resources {
    Table,
    Rid,
    Name,
    Description,
    Lid,
    CreateDatetime,
    LastUpdateDatetime,
    Level,
    Cover,
}

#[derive(DeriveIden)]
enum Libraries {
    Table,
    Lid,
}

#[derive(DeriveIden)]
enum PathMappings {
    Table,
    Pmid,
}
