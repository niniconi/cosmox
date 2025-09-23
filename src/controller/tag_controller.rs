use actix_web::{HttpResponse, Responder, delete, get, post, web};
use cosmox_macros::{ActixWebError, auto_webapi_doc};

/// Errors related to tag management.
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum TagError {
  #[error("Tag '{0}' not found.")]
  #[code(404)]
  NotFound(String),

  #[error("Not authorized to manage tags.")]
  #[code(403)]
  Unauthorized,

  #[error("Tag '{0}' already exists.")]
  #[code(409)]
  AlreadyExists(String),

  #[error("Tag name '{0}' is invalid: {1}")]
  #[code(400)]
  InvalidName(String, String),

  #[error("Maximum number of tags ({0}) reached for resource '{1}'.")]
  #[code(400)]
  MaxTagsExceeded(u32, u64),

  #[error("Tag '{0}' is protected and cannot be modified or deleted.")]
  #[code(403)]
  ProtectedTag(String),

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

#[auto_webapi_doc]
#[get("{id}")]
pub async fn get() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented {id} api")
}

#[auto_webapi_doc]
#[get("group/{id}")]
pub async fn group_get(params: web::Path<String>) -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented group/{id} api")
}

#[auto_webapi_doc]
#[post("{id}")]
pub async fn add() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented {id} api")
}

#[auto_webapi_doc]
#[post("group/add")]
pub async fn group_add() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented group/add api")
}

#[auto_webapi_doc]
#[delete("group/del")]
pub async fn group_del() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented group/del api")
}

#[auto_webapi_doc]
#[get("query")]
pub async fn query() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented query api")
}

#[auto_webapi_doc]
#[get("group/query")]
pub async fn group_query() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented group/query api")
}

#[auto_webapi_doc]
#[get("all/query")]
pub async fn all_query() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented all/query api")
}
