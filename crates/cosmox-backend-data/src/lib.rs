use std::{error::Error, sync::Arc, time::Duration};

use cosmox_configuration::Configuration;
use log::LevelFilter;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tokio::sync::OnceCell;

pub mod entities;
pub mod ipc_views;
pub mod services;

pub mod define {
    macro_rules! define_type {
        {$name:ident,$path:ident} => {
            use crate::entities::$path;
            pub type $name = $path::Model;
        };
    }

    define_type! {Device, devices}
    define_type! {LibraryPath, library_paths}
    define_type! {Library, libraries}
    define_type! {LibrariesRelatedTags, libraries_related_tags}
    define_type! {MetadataIndexes, metadata_indexes}
    define_type! {PathMapping, path_mappings}
    define_type! {Permission, permissions}
    define_type! {Resource, resources}
    define_type! {Setting, settings}
    define_type! {Role, roles}
    define_type! {Tag, tags}
    define_type! {Type, types}
    define_type! {UsersRelatedRoles, users_related_roles}
    define_type! {TagGroups, tag_groups}
    define_type! {RolesRelatedPermissions, roles_related_permissions}
    define_type! {ResourcesRelatedTags, resources_related_tags}
    define_type! {User, users}
}

#[derive(Debug, Default)]
pub struct RequestUserInner {
    pub uid: Option<u64>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}
pub type RequestUser = Arc<RequestUserInner>;

// pub trait PageHelper {} TODO

static DB_CONNECTION: OnceCell<Arc<DatabaseConnection>> = OnceCell::const_new();

pub(crate) async fn get_db_connection() -> Arc<DatabaseConnection> {
    DB_CONNECTION
        .get_or_init(|| async {
            let config = Configuration::get_global_configuration();
            let database_url = format!(
                "mysql://{}:{}@{}:{}/{}",
                config.database.user,
                config.database.password,
                config.database.host,
                config.database.port,
                config.database.database
            );

            let db_connection = {
                if let Some(database_options) = &config.database.option {
                    let mut database_opt = ConnectOptions::new(database_url);

                    database_opt
                        .max_connections(database_options.max_connections)
                        .min_connections(database_options.min_connections)
                        .connect_timeout(Duration::from_secs(database_options.connect_timeout))
                        .acquire_timeout(Duration::from_secs(database_options.acquire_timeout))
                        .idle_timeout(Duration::from_secs(database_options.idle_timeout))
                        .max_lifetime(Duration::from_secs(database_options.max_lifetime))
                        .sqlx_logging_level(LevelFilter::Debug);

                    Database::connect(database_opt).await.unwrap()
                } else {
                    Database::connect(database_url).await.unwrap()
                }
            };

            Migrator::up(&db_connection, None).await.unwrap();

            Arc::new(db_connection)
        })
        .await
        .clone()
}

pub async fn init() -> Result<(), Box<dyn Error>> {
    let _ = get_db_connection().await;
    Ok(())
}
