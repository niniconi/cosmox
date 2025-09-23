use actix_web::{HttpResponse, Responder, get, web};

use cosmox_macros::{ActixWebError, auto_webapi_doc};
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::{entities::resources, utils::message::Message};

/// Errors related to individual media file (resource) operations.
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum ResourceError {
  #[error("Resource '{0}' not found.")]
  #[code(404)]
  NotFound(u64),

  #[error("Not authorized to access resource '{0}'.")]
  #[code(403)]
  Unauthorized(u64),

  #[error("Resource with URL '{0}' already exists.")]
  #[code(409)]
  UrlConflict(String),

  #[error("Invalid resource format: {0}")]
  #[code(400)]
  InvalidFormat(String),

  #[error("Failed to parse resource content: {0}")]
  #[code(400)]
  ContentParseError(String),

  #[error("Resource '{0}' is too large; maximum size is {1} bytes.")]
  #[code(413)]
  TooLarge(u64, u64),

  #[error("Resource '{0}' cannot be deleted due to dependencies.")]
  #[code(409)]
  DeletionConflict(u64),

  #[error("Resource '{0}' is currently being processed.")]
  #[code(409)]
  ProcessingConflict(u64),

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

#[auto_webapi_doc]
#[get("{rid}")]
pub async fn get(
  rid: web::Path<u64>,
  db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, ResourceError> {
  let resource = resources::Entity::find_by_id(*rid)
    .one(db.as_ref())
    .await
    .unwrap();
  if let Some(resource) = resource {
    Ok(HttpResponse::Ok().json(Message::ok(Some(resource))))
  } else {
    Err(ResourceError::NotFound(rid.into_inner()))
  }
}

#[auto_webapi_doc]
#[get("add")]
pub async fn add() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented add api")
}

#[auto_webapi_doc]
#[get("del")]
pub async fn delete() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented del api")
}

#[auto_webapi_doc]
#[get("query")]
pub async fn query() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented query api")
}

#[auto_webapi_doc]
#[get("list")]
pub async fn list() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented list api")
}

#[auto_webapi_doc]
#[get("{rid}/metadata")]
pub async fn get_metadata() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented {rid}/metadata api")
}

#[auto_webapi_doc]
#[get("{rid}/tag/add")]
pub async fn add_tag() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented {rid}/tag/add api")
}
