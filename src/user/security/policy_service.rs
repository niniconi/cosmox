use std::sync::Arc;

use actix_web::http::header::HeaderValue;
use cosmox_macros::ActixWebError;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
  entities::{permissions, roles, users_related_roles},
  user::{
    acl_controller::AclError,
    security::auth::{self, Claims},
  },
};

#[derive(Clone)]
pub struct PolicyService {
}

/// Errors related to auth operations.
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum AuthError {
  #[error("Token is missing or invalid: {0}")]
  #[code(4001)]
  Unauthorized(String),

  #[error("Token has expired. Expiration time: {0}")]
  #[code(4002)]
  TokenExpired(String),

  #[error("User does not have permission to access this resource")]
  #[code(4003)]
  Forbidden,

  #[error("The required data for authentication is missing (e.g., Header)")]
  #[code(4004)]
  MissingData,

  #[error("User role is invalid or insufficient")]
  #[code(4005)]
  InvalidRole,

  #[error("Internal service error during authorization: {0}")]
  #[code(5001)]
  InternalError(String),
}

impl PolicyService {
  pub fn check_resource_access(
    &self,
    token: Option<&HeaderValue>,
    path: &str,
    db: Arc<DatabaseConnection>,
  ) -> Result<(), AuthError> {
    log::debug!("check resource access token: {token:?}");
    if !path.starts_with("/api") {
      Ok(())
    } else if let Some(token) = token {
      //TODO Don't use unwarp()
      match auth::verify_and_decode_jwt(token.to_str().unwrap(), auth::get_jwt_secret_key()) {
        Ok(claims) => Ok(()),
        Err(_) => Err(AuthError::Unauthorized(String::default())),
      }
    } else {
      Ok(())
    }
  }

  pub fn has_role(&self, user: &Claims, role: &str) -> bool {
    false
  }

  pub async fn add_role(db: Arc<DatabaseConnection>) {
    todo!();
  }

  pub async fn delete_role(db: Arc<DatabaseConnection>) {
    todo!();
  }

  pub async fn get_role(rid: u64, db: Arc<DatabaseConnection>) -> Result<roles::Model, AclError> {
    let role = roles::Entity::find_by_id(rid)
      .one(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| AclError::InternalError("Database error".to_string()))?;

    match role {
      Some(role) => Ok(role),
      None => Err(AclError::NotFoundRole(rid)),
    }
  }
  pub async fn get_roles_by_user(
    uid: u64,
    db: Arc<DatabaseConnection>,
  ) -> Result<Vec<roles::Model>, AclError> {
    let select =
      users_related_roles::Entity::find().filter(users_related_roles::Column::Uid.eq(uid));
    todo!();
  }
  pub async fn add_permission(db: Arc<DatabaseConnection>) {}
  pub async fn delete_permission(pid: u64, db: Arc<DatabaseConnection>) {}
  pub async fn get_permission(
    pid: u64,
    db: Arc<DatabaseConnection>,
  ) -> Result<permissions::Model, AclError> {
    let permission = permissions::Entity::find_by_id(pid)
      .one(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| AclError::InternalError("Database error".to_string()))?;

    match permission {
      Some(permission) => Ok(permission),
      None => Err(AclError::NotFoundPermission(pid)),
    }
  }

  pub async fn get_permissions_by_role(
    db: Arc<DatabaseConnection>,
  ) -> Result<Vec<permissions::Model>, AclError> {
    todo!();
  }

  pub async fn get_permissions_by_user(
    uid: u64,
    db: Arc<DatabaseConnection>,
  ) -> Result<Vec<permissions::Model>, AclError> {
    todo!();
  }

  pub async fn add_permission_for_role(db: Arc<DatabaseConnection>) {
    todo!();
  }

  pub async fn add_role_for_user(db: Arc<DatabaseConnection>) {
    todo!();
  }
}
