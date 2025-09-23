#![cfg_attr(debug_assertions, allow(unused))]
#![allow(clippy::redundant_field_names)]

use core::io::file_controller;
use std::env;

use actix_web::{App, HttpServer, web};
use configuration::Configuration;
use controller::{
  library_controller, resource_controller, system_controller, tag_controller, ui_controller,
};
use sea_orm::{Database, DatabaseConnection};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use user::user_controller;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use core::{
  plugin::plugin_controller,
  scanner::controller::{path_tree_scanner, scanner_controller},
};

use crate::core::plugin::plugin_manager::PluginManager;

pub mod configuration;
pub mod controller;
pub mod core;
pub mod entities;
pub mod services;
pub mod user;
pub mod utils;

#[derive(OpenApi)]
#[openapi(paths(
  system_controller::info,
  system_controller::restart,
  system_controller::shutdown,
  system_controller::about,
  system_controller::log,
  library_controller::get,
  library_controller::modify,
  library_controller::add,
  library_controller::delete,
  library_controller::list,
  library_controller::get_all_type,
  tag_controller::get,
  tag_controller::group_get,
  tag_controller::add,
  tag_controller::group_add,
  tag_controller::group_del,
  tag_controller::query,
  tag_controller::group_query,
  tag_controller::all_query,
  resource_controller::get,
  resource_controller::add,
  resource_controller::delete,
  resource_controller::get_metadata,
  resource_controller::query,
  resource_controller::add_tag,
  resource_controller::list,
  user_controller::get,
  user_controller::login,
  user_controller::sign_up,
  user_controller::delete,
  user_controller::query,
  user_controller::upload_avatar,
  user_controller::role_add,
  file_controller::pull,
  file_controller::push,
  scanner_controller::scan,
  scanner_controller::scan_all,
  scanner_controller::processed,
  scanner_controller::add_task,
  scanner_controller::info,
  path_tree_scanner::get_sub_path,
  plugin_controller::install_plugin,
  plugin_controller::uninstall_plugin,
  plugin_controller::enable_plugin,
  plugin_controller::disable_plugin,
  plugin_controller::info,
  ui_controller::get_core,
))]
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

  let openapi = ApiDoc::openapi();

  let database_url = format!(
    "mysql://{}:{}@{}:{}/{}",
    config.database.user,
    config.database.password,
    config.database.host,
    config.database.port,
    config.database.database
  );
  let db_connection: DatabaseConnection = Database::connect(&database_url).await.unwrap();
  let db_connection_app_data = web::Data::new(db_connection);
  let server_host = config.server.host.as_ref();
  let server_port = config.server.port;
  let config_app_data = web::Data::new(config);

  PluginManager::init();

  log::info!("start");

  #[cfg(debug_assertions)]
  println!("{}", serde_json::to_string_pretty(config).unwrap());

  HttpServer::new(move || {
    App::new()
      .app_data(db_connection_app_data.clone())
      .app_data(config_app_data.clone())
      .service(
        web::scope("api")
          .service(
            web::scope("system")
              .service(system_controller::info)
              .service(system_controller::restart)
              .service(system_controller::shutdown)
              .service(system_controller::about)
              .service(system_controller::log),
          )
          .service(
            web::scope("library")
              .service(library_controller::get)
              .service(library_controller::modify)
              .service(library_controller::add)
              .service(library_controller::delete)
              .service(library_controller::list)
              .service(library_controller::get_all_type),
          )
          .service(
            web::scope("tag")
              .service(tag_controller::get)
              .service(tag_controller::group_get)
              .service(tag_controller::add)
              .service(tag_controller::group_add)
              .service(tag_controller::group_del)
              .service(tag_controller::query)
              .service(tag_controller::group_query)
              .service(tag_controller::all_query),
          )
          .service(
            web::scope("resource")
              .service(resource_controller::get)
              .service(resource_controller::add)
              .service(resource_controller::delete)
              .service(resource_controller::get_metadata)
              .service(resource_controller::query)
              .service(resource_controller::add_tag)
              .service(resource_controller::list),
          )
          .service(
            web::scope("user")
              .service(user_controller::get)
              .service(user_controller::login)
              .service(user_controller::sign_up)
              .service(user_controller::delete)
              .service(user_controller::query)
              .service(user_controller::upload_avatar)
              .service(user_controller::role_add),
          )
          .service(
            web::scope("item")
              .service(file_controller::pull)
              .service(file_controller::push),
          )
          .service(
            web::scope("scanner")
              .service(scanner_controller::scan)
              .service(scanner_controller::scan_all)
              .service(scanner_controller::processed)
              .service(scanner_controller::add_task)
              .service(scanner_controller::info)
              .service(path_tree_scanner::get_sub_path),
          )
          .service(
            web::scope("plugin")
              .service(plugin_controller::install_plugin)
              .service(plugin_controller::uninstall_plugin)
              .service(plugin_controller::enable_plugin)
              .service(plugin_controller::disable_plugin)
              .service(plugin_controller::info),
          )
          .service(web::scope("ui").service(ui_controller::get_core)),
      )
      .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()))
      .service(actix_files::Files::new("/", "./static").index_file("index.html"))
  })
  .bind((server_host, server_port))?
  .run()
  .await
}
