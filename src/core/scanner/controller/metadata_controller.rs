use std::sync::Arc;

use actix_web::{HttpResponse, Responder, get, web};
use cosmox_macros::{ActixWebError, auto_webapi_doc};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{core::scanner::metadata::metadata_service, into_message};

#[derive(IntoParams, Deserialize)]
pub struct MetadataQueryRequest {
  pub root_node: u64,
  pub depth: usize,
}

/// Errors related to metadata operations.
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum MetadataError {
  #[error("Metadata not found with {0}")]
  #[code(404)]
  NotFound(u64),

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

#[auto_webapi_doc]
#[get("{rid}")]
pub async fn get(rid: web::Path<u64>, db: web::Data<DatabaseConnection>) -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented add api")
}

/// Query metadata from server
#[auto_webapi_doc]
#[get("query")]
pub async fn query(
  query: web::Query<MetadataQueryRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message!(
    metadata_service::query_metadata(Arc::new(query.into_inner()), db.into_inner()).await
  )
}
