use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tags::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tags::Tid)
                            .big_unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Tags::Tgid).big_unsigned().not_null())
                    .col(ColumnDef::new(Tags::Text).string().not_null())
                    .col(ColumnDef::new(Tags::CreateDatetime).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-tags-tgid")
                            .from(Tags::Table, Tags::Tgid)
                            .to(TagGroups::Table, TagGroups::Tgid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("tags_unique")
                            .table(Tags::Table)
                            .col(Tags::Tgid)
                            .col(Tags::Text)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tags::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Tags {
    Table,
    Tid,
    Tgid,
    Text,
    CreateDatetime,
}

#[derive(DeriveIden)]
enum TagGroups {
    Table,
    Tgid,
}
