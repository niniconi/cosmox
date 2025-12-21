use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use cosmox_macros::{ActixWebError, auto_webapi_doc};
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::{
  core::io::{file_controller, file_service},
  entities::path_mappings,
};
// use futures::StreamExt;

/// Errors related to file operations (upload, download, management).
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum FileError {
  #[error("File '{0}' not found.")]
  #[code(404)]
  NotFound(u64),

  #[error("Not authorized to perform file operation on '{0}'.")]
  #[code(403)]
  Unauthorized(String),

  #[error("File '{0}' already exists.")]
  #[code(409)]
  AlreadyExists(String),

  #[error("File upload failed: {0}")]
  #[code(500)]
  UploadFailed(String),

  #[error("File download failed: {0}")]
  #[code(500)]
  DownloadFailed(String),

  #[error("File '{0}' is too large; maximum size is {1} bytes.")]
  #[code(413)]
  TooLarge(String, u64),

  #[error("Invalid file type: {0}")]
  #[code(400)]
  InvalidFileType(String),

  #[error("Not supported scheme: {0}")]
  #[code(500)]
  NotSupportedScheme(String),

  #[error("Disk space exhausted: {0}")]
  #[code(507)]
  InsufficientStorage(String),

  #[error("File operation timed out for '{0}'.")]
  #[code(504)]
  // Gateway Timeout if waiting for external storage, or Request Timeout if internal file processing took too long
  OperationTimeout(String),

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

#[auto_webapi_doc]
#[post("push")]
pub async fn push() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented push api")
}

/// get item from server
#[auto_webapi_doc]
#[get("{id}/pull")]
pub async fn pull(
  file_id: web::Path<u64>,
  db: web::Data<DatabaseConnection>,
  _req: HttpRequest,
) -> impl Responder {
  file_service::pull_item_by_named_file(file_id.into_inner(), db.into_inner()).await
}
