use actix_web::{Responder, get, web};
use chrono::NaiveDateTime;
use cosmox_macros::{ActixWebError, auto_webapi_doc};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use serde_qs::web::QsQuery;
use utoipa::IntoParams;

use crate::{into_message_page, services::search_service};
use cosmox_macros::page_helper;

#[page_helper]
#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchRequest {
  pub keyword: String,
  pub tags: Option<Vec<String>>,
  pub lid: Option<u64>,
  pub before_create_datetime: Option<NaiveDateTime>,
  pub after_create_datetime: Option<NaiveDateTime>,
  pub before_last_update_datetime: Option<NaiveDateTime>,
  pub after_last_update_datetime: Option<NaiveDateTime>,
}

/// Errors related to search.
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum SearchError {
  #[error("Not authorized to manage tags.")]
  #[code(403)]
  Unauthorized,

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

#[auto_webapi_doc]
#[get("/search")]
pub async fn search(
  params: QsQuery<SearchRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message_page!(search_service::search(params.into_inner(), db.into_inner()).await)
}
