pub use sea_orm_migration::prelude::*;

mod m20260202_000001_create_path_mapping_table;
mod m20260202_000002_create_users_table;
mod m20260202_000003_create_types_table;
mod m20260202_000004_create_libraries_table;
mod m20260202_000005_create_library_path_table;
mod m20260202_000006_create_devices_table;
mod m20260202_000007_create_metadata_indexes_table;
mod m20260202_000008_create_resources_table;
mod m20260202_000009_create_tag_groups_table;
mod m20260202_000010_create_tags_table;
mod m20260202_000011_create_roles_table;
mod m20260202_000012_create_permissions_table;
mod m20260202_000013_create_settings_table;
mod m20260202_000014_create_libraries_related_tags_table;
mod m20260202_000015_create_resources_related_tags_table;
mod m20260202_000016_create_roles_related_permissions_table;
mod m20260202_000017_create_users_related_roles_table;
mod m20260216_000001_insert_roles_and_permissions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260202_000001_create_path_mapping_table::Migration),
            Box::new(m20260202_000002_create_users_table::Migration),
            Box::new(m20260202_000003_create_types_table::Migration),
            Box::new(m20260202_000004_create_libraries_table::Migration),
            Box::new(m20260202_000005_create_library_path_table::Migration),
            Box::new(m20260202_000006_create_devices_table::Migration),
            Box::new(m20260202_000008_create_resources_table::Migration),
            Box::new(m20260202_000007_create_metadata_indexes_table::Migration),
            Box::new(m20260202_000009_create_tag_groups_table::Migration),
            Box::new(m20260202_000010_create_tags_table::Migration),
            Box::new(m20260202_000011_create_roles_table::Migration),
            Box::new(m20260202_000012_create_permissions_table::Migration),
            Box::new(m20260202_000013_create_settings_table::Migration),
            Box::new(m20260202_000014_create_libraries_related_tags_table::Migration),
            Box::new(m20260202_000015_create_resources_related_tags_table::Migration),
            Box::new(m20260202_000016_create_roles_related_permissions_table::Migration),
            Box::new(m20260202_000017_create_users_related_roles_table::Migration),
            Box::new(m20260216_000001_insert_roles_and_permissions::Migration),
        ]
    }
}
