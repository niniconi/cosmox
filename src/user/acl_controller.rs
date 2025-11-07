use actix_web::{HttpResponse, Responder, delete, get, post, web};
use cosmox_macros::{ActixWebError, auto_webapi_doc, page_helper};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{into_message, user::security::policy_service::PolicyService};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleAddRequest {
  pub name: String,
  pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PermissionAddRequest {
  pub name: String,
  pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleLinkPermissionAddRequest {
  pub rid: u64,
  pub pid: u64,
}

#[page_helper]
#[derive(Deserialize, IntoParams)]
pub struct AclQueryRequest {}

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
  into_message!(PolicyService::add_role(body.into_inner(), db.into_inner()).await)
}

/// Delete role
#[auto_webapi_doc]
#[delete("role/delete")]
pub async fn delete_role(
  rid: web::Query<u64>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message!(PolicyService::delete_role(*rid, db.into_inner()).await)
}

#[auto_webapi_doc]
#[post("permission/add")]
pub async fn add_permission(
  body: web::Json<PermissionAddRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message!(PolicyService::add_permission(body.into_inner(), db.into_inner()).await)
}

#[auto_webapi_doc]
#[delete("permission/delete")]
pub async fn delete_permission(
  pid: web::Query<u64>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message!(PolicyService::delete_permission(*pid, db.into_inner()).await)
}

#[auto_webapi_doc]
#[post("role/link/permission/add")]
pub async fn add_permission_for_role(
  body: web::Json<RoleLinkPermissionAddRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message!(PolicyService::add_permission_for_role(body.pid, body.rid, db.into_inner()).await)
}

#[auto_webapi_doc]
#[get("query")]
pub async fn query(db: web::Data<DatabaseConnection>) -> impl Responder {
  HttpResponse::NotImplemented().body("NotImplemented")
}
