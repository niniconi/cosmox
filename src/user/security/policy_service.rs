use std::sync::{Arc, atomic::Ordering};

use actix_web::{
  http::{Method, header::HeaderValue},
  web::method,
};
use sea_orm::{
  ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, JoinType,
  QueryFilter, QuerySelect, RelationTrait,
};

use crate::{
  configuration::Configuration,
  entities::{permissions, roles, roles_related_permissions, users_related_roles},
  user::{
    acl_controller::{AclError, PermissionAddRequest, RoleAddRequest},
    security::{
      auth::{self, Claims},
      auth_middleware::{RequestUser, RequestUserInner},
    },
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
  fn check_permissions(path: &str, permissions: &[permissions::Model]) -> bool {
    let mut perm_iter = permissions.iter();
    match path {
      "/api/system/system.log" if !perm_iter.any(|x| x.name == "System.LogView") => false,
      "/api/system/delete/all" if !perm_iter.any(|x| x.name == "System.Wipe") => false,
      "/api/system/shutdown" | "/api/system/restart"
        if !perm_iter.any(|x| x.name == "System.Power") =>
      {
        false
      }

      "/api/plugin/install" if !perm_iter.any(|x| x.name == "Plugin.Install") => false,
      "/api/plugin/uninstall" if !perm_iter.any(|x| x.name == "Plugin.Uninstall") => false,
      "/api/plugin/enable" | "/api/plugin/disable"
        if !perm_iter.any(|x| x.name == "Plugin.Manage") =>
      {
        false
      }

      "/api/metadata/query" if !perm_iter.any(|x| x.name == "Metadata.View") => false,
      "/api/user/delete" if !perm_iter.any(|x| x.name == "User.Delete") => false,
      "/api/user/signUp" if !perm_iter.any(|x| x.name == "User.Create") => false,
      "/api/user/link/role/add" if !perm_iter.any(|x| x.name == "User.ManageRoles") => false,
      "/api/tag/add" | "/api/tag/group/add" | "/api/tag/group/delete"
        if !perm_iter.any(|x| x.name == "Tag.Manage") =>
      {
        false
      }
      s if s.starts_with("/api/acl/role") && !perm_iter.any(|x| x.name == "User.ManageRoles") => {
        false
      }
      s if s.starts_with("/api/acl/permission")
        && !perm_iter.any(|x| x.name == "User.ManagePerms") =>
      {
        false
      }
      s if s.starts_with("/api/scanner") && !perm_iter.any(|x| x.name == "Library.Scan") => false,
      _ => true,
    }
  }

  pub async fn check_resource_access(
    &self,
    token: Option<HeaderValue>,
    path: String,
    method: Method,
    db: Arc<DatabaseConnection>,
  ) -> Result<RequestUser, AuthError> {
    log::debug!("check resource access token: {token:?}");

    let is_first_boot = Configuration::get_global_configuration().await
      .state
      .is_first_boot
      .load(Ordering::Relaxed);

    let is_white_listed = match ((&method, &path[..])) {
      (&Method::OPTIONS, _) => true,
      (_, p) if !p.starts_with("/api") => true,
      (&Method::POST, "/api/user/login") => true,
      (&Method::GET, "/api/system/info") if is_first_boot => true,
      (&Method::POST, "/api/initialize") => {
        if is_first_boot {
          true
        } else {
          return Err(AuthError::Forbidden);
        }
      }
      _ => false,
    };

    if is_white_listed {
      Ok(Arc::new(RequestUserInner {
        uid: None,
        roles: vec!["Anonymous".to_string()],
        permissions: vec![],
      }))
    } else if let Some(token) = token {
      let Ok(token) = token.to_str() else {
        log::error!(
          "Failed to convert auth token `HeaderValue` to `String`: invalid utf-8 or non-ascii characters"
        );
        return Err(AuthError::InternalError("Internal error".to_string()));
      };

      match auth::verify_and_decode_jwt(token, auth::get_jwt_secret_key()) {
        Ok(claims) => {
          let Ok(uid): Result<u64, _> = claims.sub.parse() else {
            log::error!(
              "Failed to parse claims sub: expected integer, got '{}'",
              claims.sub
            );
            return Err(AuthError::Unauthorized(token.to_string()));
          };

          log::info!("user: {uid} access api: {path}");

          let roles = match PolicyService::get_roles_by_user(uid, db.clone()).await {
            Ok(role) => role,
            Err(err) => return Err(AuthError::InternalError("Internal error".to_string())),
          };
          let permissions = match PolicyService::get_permissions_by_user(uid, db).await {
            Ok(permissions) => permissions,
            Err(err) => return Err(AuthError::InternalError("Internal error".to_string())),
          };

          let perm_check = Self::check_permissions(&path[..], &permissions);

          if perm_check {
            Ok(Arc::new(RequestUserInner {
              uid: Some(uid),
              roles: roles.iter().map(|x| x.name.clone()).collect(),
              permissions: permissions.iter().map(|x| x.name.clone()).collect(),
            }))
          } else {
            Err(AuthError::Forbidden)
          }
        }
        Err(_) => Err(AuthError::Unauthorized(token.to_string())),
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
      Err(_err) => Err(AclError::InternalError("Database error".to_string())),
    }
  }

  pub async fn add_permission(
    permission: PermissionAddRequest,
    db: Arc<DatabaseConnection>,
  ) -> Result<(), AclError> {
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

    select
      .all(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| AclError::InternalError("Database error".to_string()))
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
    select
      .all(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| AclError::InternalError("Database error".to_string()))
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
