use actix_web::{HttpResponse, Responder, delete, post, web};
use cosmox_macros::{ActixWebError, auto_webapi_doc};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleAddRequest {}

#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum AclError {
  #[error("Role '{0}' not found.")]
  #[code(404)]
  NotFoundRole(u64),

  #[error("Permission '{0}' not found.")]
  #[code(404)]
  NotFoundPermission(u64),

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

/// Add role
///
/// Create a new user
#[auto_webapi_doc]
#[post("role/add")]
pub async fn add_role(
  body: web::Json<RoleAddRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  HttpResponse::NotImplemented().body("NotImplemented")
}

/// Delete role
#[auto_webapi_doc]
#[delete("role/delete")]
pub async fn delete_role(
  uid: web::Query<u64>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  HttpResponse::NotImplemented().body("NotImplemented")
}
