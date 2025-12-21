use actix_web::{HttpResponse, Responder, delete, get, post, web};
use cosmox_macros::{ActixWebError, auto_webapi_doc, page_helper};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::{IntoParams, ToSchema};

use crate::core::plugin::plugin_manager::PluginManager;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InstallPluginParams {
  pub url: Option<String>,
}

#[page_helper]
#[derive(Debug, Deserialize, IntoParams)]
pub struct PluginQueryRequest {}

/// Errors related to plugin management and execution.
#[derive(Debug, Error, ActixWebError)]
pub enum PluginError {
  #[error("Plugin '{0}' not found.")]
  #[code(404)]
  NotFound(String),

  #[error("Not authorized to manage plugins.")]
  #[code(403)]
  Unauthorized,

  #[error("Plugin '{0}' is already installed.")]
  #[code(409)]
  AlreadyInstalled(String),

  #[error("Plugin '{0}' failed to download from '{1}'.")]
  #[code(504)]
  DownloadTimeout(String, String),

  #[error("Plugin '{0}' is not compatible with current system version '{1}'.")]
  #[code(400)]
  IncompatibleVersion(String, String),

  #[error("Plugin '{0}' failed to load: {1}")]
  #[code(500)]
  LoadFailed(String, String),

  #[error("Plugin execution failed for '{0}': {1}")]
  #[code(500)]
  ExecutionFailed(String, String),

  #[error("Plugin '{0}' is disabled.")]
  #[code(403)]
  Disabled(String),

  #[error("Plugin registration failed due to missing manifest.")]
  #[code(400)]
  MissingManifest,

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

#[auto_webapi_doc]
#[post("install")]
pub async fn install_plugin(body: web::Json<InstallPluginParams>) -> impl Responder {
  HttpResponse::NotImplemented().body(format!(
    "Install plugin api is not yet implemented. payload = {body:#?}"
  ))
}

#[auto_webapi_doc]
#[delete("uninstall")]
pub async fn uninstall_plugin() -> impl Responder {
  HttpResponse::NotImplemented().body("Uninstall plugin api is not yet implemented.")
}

#[auto_webapi_doc]
#[delete("enable")]
pub async fn enable_plugin() -> impl Responder {
  HttpResponse::NotImplemented().body("Enable plugin api is not yet implemented.")
}

#[auto_webapi_doc]
#[delete("disable")]
pub async fn disable_plugin() -> impl Responder {
  HttpResponse::NotImplemented().body("Disable plugin api is not yet implemented.")
}

#[auto_webapi_doc]
#[get("info")]
pub async fn info() -> impl Responder {
  #[cfg(debug_assertions)]
  return HttpResponse::Ok().body(format!("{:#?}", PluginManager::get_plugin_manager()));
  #[cfg(not(debug_assertions))]
  return HttpResponse::Ok().body("");
}
