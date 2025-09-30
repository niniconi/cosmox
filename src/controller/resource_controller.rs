use std::sync::Arc;

use actix_web::{HttpResponse, Responder, delete, get, post, web};

use cosmox_macros::{ActixWebError, auto_webapi_doc, page_helper};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
  entities::{resources, tags},
  into_message, into_message_page,
  services::resource_service,
  utils::message::Message,
};

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

#[derive(Serialize, Deserialize, IntoParams)]
struct ResourceDeleteRequest {
  rid: u64,
}

#[page_helper]
#[derive(Deserialize, IntoParams)]
pub struct ResourceQueryRequest {}

#[derive(Deserialize, ToSchema)]
pub struct ResourceAddTagRequest {
  tags: Vec<u64>,
}

#[auto_webapi_doc]
#[get("{rid}")]
pub async fn get(rid: web::Path<u64>, db: web::Data<DatabaseConnection>) -> impl Responder {
  into_message!(resource_service::get_resource(*rid, db.into_inner()).await)
}

#[auto_webapi_doc]
#[get("add")]
pub async fn add() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented add api")
}

#[auto_webapi_doc]
#[delete("delete")]
pub async fn delete(
  rid: web::Query<ResourceDeleteRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message!(resource_service::delete_resource(rid.rid, db.into_inner()).await)
}

#[auto_webapi_doc]
#[get("query")]
pub async fn query(
  params: web::Query<ResourceQueryRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message_page!(resource_service::query_resources(params.into_inner(), db.into_inner()).await)
}

#[auto_webapi_doc]
#[get("{rid}/metadata")]
pub async fn get_metadata() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented {rid}/metadata api")
}

#[auto_webapi_doc]
#[post("{rid}/tag/add")]
pub async fn add_tag(
  rid: web::Path<u64>,
  params: web::Json<ResourceAddTagRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message!(
    resource_service::add_tags_for_resource(*rid, params.tags.clone(), db.into_inner()).await
  )
}
