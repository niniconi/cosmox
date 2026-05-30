use cosmox_macros::page_helper;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, JoinType, QueryFilter,
    QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    entities::{permissions, roles, roles_related_permissions, users_related_roles},
    get_db_connection,
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

#[derive(Debug, thiserror::Error)]
pub enum AclError {
    #[error("Role '{0}' not found.")]
    NotFoundRole(u64),

    #[error("Permission '{0}' not found.")]
    NotFoundPermission(u64),

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

#[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct RoleAddRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct PermissionAddRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleLinkPermissionAddRequest {
    pub rid: u64,
    pub pid: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserRoleAddRequest {
    pub uid: u64,
    pub rid: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleDeleteRequest {
    pub rid: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionDeleteRequest {
    pub pid: u64,
}

#[page_helper]
#[derive(Deserialize)]
pub struct AclQueryRequest {}

// impl PolicyService {

pub async fn add_role(role: RoleAddRequest) -> Result<(), AclError> {
    let db = get_db_connection().await;
    let role = roles::ActiveModel {
        name: Set(role.name),
        ..Default::default()
    };

    role.insert(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|_err| AclError::InternalError("Database error".to_string()))?;

    Ok(())
}

pub async fn delete_role(rid: u64) -> Result<(), AclError> {
    let db = get_db_connection().await;
    roles::Entity::delete_by_id(rid)
        .exec(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|_err| AclError::InternalError("Database error".to_string()))?;
    Ok(())
}

pub async fn get_role(rid: u64) -> Result<roles::Model, AclError> {
    let db = get_db_connection().await;
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

pub async fn get_roles_by_user(uid: u64) -> Result<Vec<roles::Model>, AclError> {
    let db = get_db_connection().await;
    let select = roles::Entity::find()
        .join(
            JoinType::InnerJoin,
            roles::Relation::UsersRelatedRoles.def(),
        )
        .filter(users_related_roles::Column::Uid.eq(uid));

    match select.all(db.as_ref()).await {
        Ok(roles) => Ok(roles),
        Err(err) => {
            log::error!("Failed to query roles for user {uid}: {err}");
            Err(AclError::InternalError("Database error".to_string()))
        }
    }
}

pub async fn add_permission(permission: PermissionAddRequest) -> Result<(), AclError> {
    let db = get_db_connection().await;
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

pub async fn delete_permission(pid: u64) -> Result<(), AclError> {
    let db = get_db_connection().await;
    permissions::Entity::delete_by_id(pid)
        .exec(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|_err| AclError::InternalError("Database error".to_string()))?;
    Ok(())
}

pub async fn get_permission(pid: u64) -> Result<permissions::Model, AclError> {
    let db = get_db_connection().await;
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

pub async fn get_permissions_by_role(rid: u64) -> Result<Vec<permissions::Model>, AclError> {
    let db = get_db_connection().await;
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

pub async fn get_permissions_by_user(uid: u64) -> Result<Vec<permissions::Model>, AclError> {
    let db = get_db_connection().await;
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

pub async fn add_permission_for_role(pid: u64, rid: u64) -> Result<(), AclError> {
    let db = get_db_connection().await;
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

pub async fn add_role_for_user(rid: u64, uid: u64) -> Result<(), AclError> {
    let db = get_db_connection().await;
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

pub async fn query_role() -> Result<Vec<roles::Model>, AclError> {
    let db = get_db_connection().await;
    roles::Entity::find()
        .all(db.as_ref())
        .await
        .inspect_err(|err| log::error!("Failed to query roles: {err}"))
        .map_err(|_err| AclError::InternalError("Database error".to_string()))
}

pub async fn query_permission() -> Result<Vec<permissions::Model>, AclError> {
    let db = get_db_connection().await;
    permissions::Entity::find()
        .all(db.as_ref())
        .await
        .inspect_err(|err| log::error!("Failed to query permissions: {err}"))
        .map_err(|_err| AclError::InternalError("Database error".to_string()))
}
// }
