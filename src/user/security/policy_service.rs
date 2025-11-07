use std::sync::Arc;

use actix_web::http::{Method, header::HeaderValue};
use cosmox_macros::ActixWebError;
use futures::TryFutureExt;
use sea_orm::{
  ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityOrSelect, EntityTrait,
  JoinType, QueryFilter, QuerySelect, RelationTrait,
};

use crate::{
  entities::{
    permissions, prelude::UsersRelatedRoles, roles, roles_related_permissions, users_related_roles,
  },
  user::{
    acl_controller::{AclError, PermissionAddRequest, RoleAddRequest},
    security::auth::{self, Claims},
  },
};

#[derive(Clone)]
pub struct PolicyService {}

/// Errors related to auth operations.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
  #[error("Token is missing or invalid: {0}")]
  Unauthorized(String),

  #[error("Token has expired. Expiration time: {0}")]
  TokenExpired(String),

  #[error("User does not have permission to access this resource")]
  Forbidden,

  #[error("The required data for authentication is missing (e.g., Header)")]
  MissingData,

  #[error("User role is invalid or insufficient")]
  InvalidRole,

  #[error("Internal service error during authorization: {0}")]
  InternalError(String),
}

impl PolicyService {
  pub async fn check_resource_access(
    &self,
    token: Option<HeaderValue>,
    path: String,
    method: Method,
    db: Arc<DatabaseConnection>,
  ) -> Result<(), AuthError> {
    log::debug!("check resource access token: {token:?}");
    if !path.starts_with("/api")
      || path.starts_with("/api-docs")
      || (path == "/api/user/login" && method == Method::POST)
    {
      Ok(())
    } else if let Some(token) = token {
      //TODO Don't use unwarp()
      match auth::verify_and_decode_jwt(token.to_str().unwrap(), auth::get_jwt_secret_key()) {
        Ok(claims) => {
          let uid: u64 = match claims.sub.parse() {
            Ok(uid) => uid,
            Err(_err) => {
              return Err(AuthError::Unauthorized("Invalid token".to_string()));
            }
          };

          log::info!("user: {uid} access api: {path}");

          let permissions = PolicyService::get_permissions_by_user(uid, db).await;
          // TODO check permission
          /*
          match path {
            path if path.starts_with("/api/user") => {
              todo!()
            }
            path if path.starts_with("/api/plugin") => {
              todo!()
            }
            path if path.starts_with("/api/system") => {
              todo!()
            }
            _ => todo!(),
          }
          */

          Ok(())
        }
        Err(_) => Err(AuthError::Unauthorized(String::default())),
      }
    } else {
      Err(AuthError::Unauthorized(String::default()))
    }
  }

  pub fn has_role(&self, user: &Claims, role: &str) -> bool {
    false
  }

  pub async fn add_role(role: RoleAddRequest, db: Arc<DatabaseConnection>) -> Result<(), AclError> {
    let role = roles::ActiveModel {
      name: Set(role.name),
      ..Default::default()
    };

    role
      .insert(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| AclError::InternalError("Database error".to_string()))?;

    Ok(())
  }

  pub async fn delete_role(rid: u64, db: Arc<DatabaseConnection>) -> Result<(), AclError> {
    roles::Entity::delete_by_id(rid)
      .exec(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| AclError::InternalError("Database error".to_string()))?;
    Ok(())
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
    let select = roles::Entity::find()
      .join(
        JoinType::InnerJoin,
        roles::Relation::UsersRelatedRoles.def(),
      )
      .filter(users_related_roles::Column::Uid.eq(uid));

    match select.all(db.as_ref()).await {
      Ok(roles) => Ok(roles),
      Err(err) => Err(AclError::InternalError("Database error".to_string())),
    }
  }

  pub async fn add_permission(permission: PermissionAddRequest, db: Arc<DatabaseConnection>) -> Result<(), AclError> {
    let permission = permissions::ActiveModel {
      name: Set(permission.name),
      description: Set(permission.description),
      ..Default::default()
    };

    permission
      .insert(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| AclError::InternalError("Database error".to_string()))?;
    Ok(())
  }

  pub async fn delete_permission(pid: u64, db: Arc<DatabaseConnection>) -> Result<(), AclError> {
    permissions::Entity::delete_by_id(pid)
      .exec(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| AclError::InternalError("Database error".to_string()))?;
    Ok(())
  }

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
    rid: u64,
    db: Arc<DatabaseConnection>,
  ) -> Result<Vec<permissions::Model>, AclError> {
    let select = permissions::Entity::find()
      .join(
        JoinType::InnerJoin,
        permissions::Relation::RolesRelatedPermissions.def(),
      )
      .filter(roles_related_permissions::Column::Rid.eq(rid));

    match select.all(db.as_ref()).await {
      Ok(permissions) => Ok(permissions),
      Err(err) => Err(AclError::InternalError("Database error".to_string())),
    }
  }

  pub async fn get_permissions_by_user(
    uid: u64,
    db: Arc<DatabaseConnection>,
  ) -> Result<Vec<permissions::Model>, AclError> {
    let select = permissions::Entity::find()
      .join(
        JoinType::InnerJoin,
        permissions::Relation::RolesRelatedPermissions.def(),
      )
      .join(
        JoinType::InnerJoin,
        roles_related_permissions::Relation::Roles.def(),
      )
      .join(
        JoinType::InnerJoin,
        roles::Relation::UsersRelatedRoles.def(),
      )
      .filter(users_related_roles::Column::Uid.eq(uid));
    match select.all(db.as_ref()).await {
      Ok(permissions) => Ok(permissions),
      Err(err) => Err(AclError::InternalError("Database error".to_string())),
    }
  }

  pub async fn add_permission_for_role(
    pid: u64,
    rid: u64,
    db: Arc<DatabaseConnection>,
  ) -> Result<(), AclError> {
    let role_permission_relation = roles_related_permissions::ActiveModel {
      rid: Set(rid),
      pid: Set(pid),
      ..Default::default()
    };
    role_permission_relation
      .insert(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| AclError::InternalError("Database error".to_string()))?;
    Ok(())
  }

  pub async fn add_role_for_user(
    rid: u64,
    uid: u64,
    db: Arc<DatabaseConnection>,
  ) -> Result<(), AclError> {
    let user_role_relation = users_related_roles::ActiveModel {
      uid: Set(uid),
      rid: Set(rid),
      ..Default::default()
    };
    user_role_relation
      .insert(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| AclError::InternalError("Database error".to_string()))?;
    Ok(())
  }

  pub async fn query_role(db: Arc<DatabaseConnection>) -> Result<Vec<roles::Model>, AclError> {
    todo!()
  }

  pub async fn query_permission(
    db: Arc<DatabaseConnection>,
  ) -> Result<Vec<permissions::Model>, AclError> {
    todo!()
  }
}
