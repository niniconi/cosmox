#![cfg_attr(debug_assertions, allow(unused))]
#![allow(clippy::redundant_field_names)]

use core::io::file_controller;
use std::{env, time::Duration};

use actix_cors::Cors;
use actix_web::{App, HttpServer, get, http::header, web};
use configuration::Configuration;
use controller::{
  library_controller, resource_controller, system_controller, tag_controller, ui_controller,
};
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use user::user_controller;
use utoipa::{OpenApi, openapi};
use utoipa_actix_web::{AppExt, scope};
use utoipa_scalar::{Scalar, Servable};

use core::{
  plugin::plugin_controller,
  scanner::controller::{path_tree_scanner, scanner_controller},
};

use crate::{
  controller::init,
  core::{plugin::plugin_manager::PluginManager, scanner::controller::metadata_controller},
  user::{
    acl_controller,
    security::{auth_middleware::TokenAuth, policy_service::PolicyService},
  },
};

pub mod configuration;
pub mod controller;
pub mod core;
pub mod entities;
pub mod services;
pub mod user;
pub mod utils;

#[derive(OpenApi)]
struct ApiDoc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  let config = Configuration::get_global_configuration();

  println!(include_str!("../banner.txt"), env!("CARGO_PKG_VERSION"));

  let file_appender =
    RollingFileAppender::new(Rotation::DAILY, config.cosmox.log.path.as_str(), "app.log");
  let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);

  tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new(
      env::var("RUST_LOG").unwrap_or_else(
        #[cfg(debug_assertions)]
        |_| "trace".into(),
        #[cfg(not(debug_assertions))]
        |_| "info".into(),
      ),
    ))
    .with(tracing_subscriber::fmt::layer())
    .with(tracing_subscriber::fmt::layer().with_writer(non_blocking_appender))
    .init();

  let database_url = format!(
    "mysql://{}:{}@{}:{}/{}",
    config.database.user,
    config.database.password,
    config.database.host,
    config.database.port,
    config.database.database
  );

  let db_connection: DatabaseConnection = if let Some(database_options) = &config.database.option {
    let mut database_opt = ConnectOptions::new(database_url);

    database_opt
      .max_connections(database_options.max_connections)
      .min_connections(database_options.min_connections)
      .connect_timeout(Duration::from_secs(database_options.connect_timeout))
      .acquire_timeout(Duration::from_secs(database_options.acquire_timeout))
      .idle_timeout(Duration::from_secs(database_options.idle_timeout))
      .max_lifetime(Duration::from_secs(database_options.max_lifetime));

    Database::connect(database_opt).await.unwrap()
  } else {
    Database::connect(database_url).await.unwrap()
  };

  Migrator::up(&db_connection, None).await.unwrap();

  let db_connection_app_data = web::Data::new(db_connection);
  let server_host = config.server.host.as_ref();
  let server_port = config.server.port;
  let config_app_data = web::Data::new(config);

  PluginManager::start().await;

  let policy_service = web::Data::new(PolicyService {});

  log::info!("start");

  #[cfg(debug_assertions)]
  println!("{}", serde_json::to_string_pretty(config).unwrap());

  HttpServer::new(move || {
    let cors = Cors::default()
      .allow_any_origin()
      .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
      .allowed_headers(vec![
        header::AUTHORIZATION,
        header::ACCEPT,
        header::CONTENT_TYPE,
        header::RANGE,
        header::IF_RANGE,
        header::CACHE_CONTROL,
        header::PRAGMA,
        header::HeaderName::from_static("x-session-id"),
        header::HeaderName::from_static("x-client-version"),
      ])
      .expose_headers(vec![
        header::CONTENT_RANGE,
        header::CONTENT_LENGTH,
        header::ACCEPT_RANGES,
        header::SERVER,
      ])
      .max_age(3600);

    let (app, api) = App::new()
      .app_data(db_connection_app_data.clone())
      .app_data(config_app_data.clone())
      .app_data(policy_service.clone())
      .wrap(cors)
      .wrap(TokenAuth)
      .into_utoipa_app()
      .openapi(ApiDoc::openapi())
      .service(
        scope::scope("/api")
          .service(init::initialize)
          .service(
            scope::scope("/system")
              .service(system_controller::info)
              .service(system_controller::restart)
              .service(system_controller::shutdown)
              .service(system_controller::about)
              .service(system_controller::log),
          )
          .service(
            scope::scope("/library")
              .service(library_controller::modify)
              .service(library_controller::add)
              .service(library_controller::delete)
              .service(library_controller::query)
              .service(library_controller::get_all_type)
              .service(library_controller::get),
          )
          .service(
            scope::scope("/tag")
              .service(tag_controller::group_get)
              .service(tag_controller::add)
              .service(tag_controller::group_add)
              .service(tag_controller::group_delete)
              .service(tag_controller::query)
              .service(tag_controller::group_query)
              .service(tag_controller::all_query)
              .service(tag_controller::get),
          )
          .service(
            scope::scope("/resource")
              .service(resource_controller::add)
              .service(resource_controller::delete)
              .service(resource_controller::get_metadata)
              .service(resource_controller::query)
              .service(resource_controller::add_tag)
              .service(resource_controller::get),
          )
          .service(
            scope::scope("/user")
              .service(user_controller::login)
              .service(user_controller::sign_up)
              .service(user_controller::delete)
              .service(user_controller::query)
              .service(user_controller::upload_avatar)
              .service(user_controller::role_add)
              .service(user_controller::get)
              .service(
                utoipa_actix_web::scope::scope("/acl")
                  .service(acl_controller::add_role)
                  .service(acl_controller::delete_role)
                  .service(acl_controller::add_permission)
                  .service(acl_controller::delete_permission)
                  .service(acl_controller::add_permission_for_role),
              ),
          )
          .service(
            scope::scope("/item")
              .service(file_controller::pull)
              .service(file_controller::push),
          )
          .service(
            scope::scope("/scanner")
              .service(scanner_controller::scan_all)
              .service(scanner_controller::scan)
              .service(scanner_controller::processed)
              .service(scanner_controller::add_task)
              .service(scanner_controller::info)
              .service(path_tree_scanner::get_sub_path),
          )
          .service(
            scope::scope("/plugin")
              .service(plugin_controller::install_plugin)
              .service(plugin_controller::uninstall_plugin)
              .service(plugin_controller::enable_plugin)
              .service(plugin_controller::disable_plugin)
              .service(plugin_controller::info),
          )
          .service(
            scope::scope("/metadata")
              .service(metadata_controller::query)
              .service(metadata_controller::get),
          )
          .service(scope::scope("/ui").service(ui_controller::get_core)),
      )
      .split_for_parts();

    app
      .service(Scalar::with_url("/scalar", api))
      .service(actix_files::Files::new("/", "./static").index_file("index.html"))
  })
  .bind((server_host, server_port))?
  .run()
  .await
}
