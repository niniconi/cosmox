use crate::configuration::Configuration;
use actix_web::{HttpResponse, Responder, delete, get};
use cosmox_macros::{ActixWebError, auto_webapi_doc};
use serde::{Deserialize, Serialize};
use std::{
  fs::{self, File},
  io::Read,
  process::exit,
};
use sysinfo::System;

#[derive(Serialize, Deserialize)]
struct SystemInfo {
  os: String,
  name: &'static str,
  version: &'static str,
  author: &'static str,
}

/// Errors related to system management operations (e.g., shutdown, restart).
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum SystemError {
  #[error("System resource '{0}' not found.")]
  #[code(404)]
  NotFound(String),

  #[error("Not authorized to perform system operation: {0}.")]
  #[code(403)]
  Unauthorized(String),

  #[error("System is already in state '{0}'.")]
  #[code(409)]
  AlreadyInState(String),

  #[error("System operation '{0}' failed: {1}")]
  #[code(500)]
  OperationFailed(String, String),

  #[error("Invalid system state for operation '{0}': current state is '{1}'.")]
  #[code(400)]
  InvalidState(String, String),

  #[error("System configuration invalid: {0}")]
  #[code(500)]
  ConfigurationError(String),

  #[error("System shutdown initiated.")]
  #[code(202)]
  ShutdownInitiated, // Specific for async operations that might not immediately fail

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

#[auto_webapi_doc]
#[get("info")]
pub async fn info() -> impl Responder {
  // let sys = System::new_all();
  let info = SystemInfo {
    os: System::name().unwrap_or(String::from("Unknown")),
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    author: env!("CARGO_PKG_AUTHORS"),
  };
  HttpResponse::Ok().json(info)
}

/// Restart server
#[auto_webapi_doc]
#[get("restart")]
pub async fn restart() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented restart api")
}

#[auto_webapi_doc]
#[get("shutdown")]
pub async fn shutdown() -> impl Responder {
  exit(0);

  #[warn(dead_code)]
  HttpResponse::Ok().body("unreachable")
}

#[auto_webapi_doc]
#[get("about.md")]
pub async fn about() -> impl Responder {
  let mut readme = match File::open("./README.md") {
    Ok(file) => file,
    Err(err) => return HttpResponse::InternalServerError().body(format!("{err}")),
  };
  let metadata = match readme.metadata() {
    Ok(metadata) => metadata,
    Err(err) => return HttpResponse::InternalServerError().body(format!("{err}")),
  };
  let mut reuslt = String::with_capacity(metadata.len() as usize);
  match readme.read_to_string(&mut reuslt) {
    Ok(_) => HttpResponse::Ok().body(reuslt),
    Err(err) => HttpResponse::InternalServerError().body(format!("{err}")),
  }
}

#[auto_webapi_doc]
#[get("system.log")]
pub async fn log() -> impl Responder {
  let log_path = &Configuration::get_global_configuration().cosmox.log.path;
  let log = fs::read_to_string(log_path);
  HttpResponse::Ok().body(log.unwrap())
}

#[auto_webapi_doc]
#[delete("delete/all")]
pub async fn delete_all() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented delete/all api")
}
