use actix_web::{HttpResponse, Responder, delete, get, post, web};
use cosmox_macros::{ActixWebError, auto_webapi_doc, page_helper};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{into_message, into_message_page, services::tag_service};

#[page_helper]
#[derive(Debug, Deserialize, IntoParams)]
pub struct TagQueryRequest {
  tid: Option<u64>,
}

/// Errors related to tag management.
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum TagError {
  #[error("Tag '{0}' not found.")]
  #[code(404)]
  NotFound(u64),

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
#[get("/{id}")]
pub async fn get(
  tid: web::Path<u64>,
  db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, TagError> {
  into_message!(tag_service::get_tag(*tid, db.into_inner()).await)
}

#[auto_webapi_doc]
#[get("/group/{id}")]
pub async fn group_get(params: web::Path<String>) -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented group/{id} api")
}

#[auto_webapi_doc]
#[post("/add")]
pub async fn add(tid: web::Path<u64>, db: web::Data<DatabaseConnection>) -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented add api")
}

#[auto_webapi_doc]
#[post("/group/add")]
pub async fn group_add() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented group/add api")
}

#[auto_webapi_doc]
#[delete("/group/delete")]
pub async fn group_delete() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented group/del api")
}

#[auto_webapi_doc]
#[get("/query")]
pub async fn query(
  params: web::Query<TagQueryRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message_page!(tag_service::query_tag(params.into_inner(), db.into_inner()).await)
}

#[auto_webapi_doc]
#[get("/group/query")]
pub async fn group_query() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented group/query api")
}

#[auto_webapi_doc]
#[get("/all/query")]
pub async fn all_query() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented all/query api")
}
