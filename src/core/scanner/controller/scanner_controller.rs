use std::sync::{LazyLock, RwLock};

use actix_web::{HttpResponse, Responder, get, http::StatusCode, post, web};

use cosmox_macros::{ActixWebError, auto_webapi_doc};
use futures::FutureExt;
use sea_orm::DatabaseConnection;

use crate::{
  core::scanner::scanner_manager::{self, start_scanner},
  utils::message::Message,
};

#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum ScannerError {
  #[error("Target library '{0}' not found.")]
  #[code(404)]
  NotFound(u64),

  #[error("Not authorized to manage scanners.")]
  #[code(403)]
  Unauthorized,

  #[error("Scanner '{0}' is already running.")]
  #[code(409)]
  AlreadyRunning(String),

  #[error("Scanner '{0}' failed to start: {1}")]
  #[code(500)]
  StartFailed(String, String),

  #[error("Scanner task for '{0}' failed: {1}")]
  #[code(500)]
  TaskFailed(String, String),

  #[error("Scanner '{0}' is not configured correctly: {1}")]
  #[code(400)]
  InvalidConfiguration(String, String),

  #[error("Scan path '{0}' is invalid or inaccessible.")]
  #[code(400)]
  InvalidScanPath(String),

  #[error("No available scanner instances to process the request.")]
  #[code(503)]
  NoAvailableScanner,

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

pub enum ScannerStatus {
  Runing { task_count: usize, completed: usize },
  Stop,
  Err(ScannerError),
}

static SCANNER_STATE: LazyLock<RwLock<ScannerStatus>> =
  LazyLock::new(|| RwLock::new(ScannerStatus::Stop));

/// Scan library by id
/// TODO web::block can't not run as expected.
#[auto_webapi_doc]
#[get("scan/{lid}")]
pub async fn scan(
  lid: web::Path<u64>,
  db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, ScannerError> {
  // let _ = web::block(async move || {
  let context = scanner_manager::prepare_context_information(
    scanner_manager::SelectedLibraries::SINGLE(*lid),
    db.into_inner(),
  )
  .await;

  match context {
    Ok(contexts) => {
      if let Some(context) = contexts.first() {
        log::debug!("start scanner by context {context:#?}");
        let _ = start_scanner(context.clone()).await;
      } else {
        *SCANNER_STATE.write().unwrap() = ScannerStatus::Err(ScannerError::NotFound(*lid))
      }
    }
    Err(scanner_error) => *SCANNER_STATE.write().unwrap() = ScannerStatus::Err(scanner_error),
  }
  // })
  // .await
  // .unwrap();

  Ok(HttpResponse::Ok().json(Message::ok(Some("start"))))
}

/// Scan all libraries
/// TODO web::block can't not run as expected.
#[auto_webapi_doc]
#[get("scan/all")]
pub async fn scan_all(db: web::Data<DatabaseConnection>) -> Result<impl Responder, ScannerError> {
  web::block(async || {
    let context = scanner_manager::prepare_context_information(
      scanner_manager::SelectedLibraries::ALL,
      db.into_inner(),
    )
    .await;

    match context {
      Ok(contexts) => {
        for context in contexts {
          log::debug!("start scanner by context {context:#?}");
          start_scanner(context.clone());
        }
      }
      Err(scanner_error) => *SCANNER_STATE.write().unwrap() = ScannerStatus::Err(scanner_error),
    }
  })
  .await
  .unwrap();

  Ok(HttpResponse::Ok().json(Message::ok(Some("start"))))
}

#[auto_webapi_doc]
#[get("add/task")]
pub async fn add_task() -> Result<impl Responder, ScannerError> {
  Ok(HttpResponse::NotImplemented().body("Add task api is not yet implemented."))
}

/// get status of scanner
///
#[auto_webapi_doc]
#[get("status")]
pub async fn processed() -> Result<impl Responder, ScannerError> {
  Ok(HttpResponse::NotImplemented().body("Get processed api is not yet implemented."))
}

/// get information of scanner
///
#[auto_webapi_doc]
#[post("info")]
pub async fn info() -> Result<impl Responder, ScannerError> {
  Ok(HttpResponse::NotImplemented().body("Get scanner info api is not yet implemented."))
}
