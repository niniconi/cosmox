use cosmox_backend_data::services::role_permission_service;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};

pub use cosmox_backend_data::{
    define::{Permission, Role},
    services::role_permission_service::{
        AclError, PermissionAddRequest, PermissionDeleteRequest, RoleAddRequest, RoleDeleteRequest,
        RoleLinkPermissionAddRequest, UserRoleAddRequest,
    },
};

pub async fn add_role(
    ctx: &mut Context<'_>,
    payload: RoleAddRequest,
) -> Result<Message<()>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::AddRole;
    Message::from_service(ctx, role_permission_service::add_role(payload)).await
}

pub async fn delete_role(
    ctx: &mut Context<'_>,
    rid: u64,
) -> Result<Message<()>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::DeleteRole { rid };
    Message::from_service(ctx, role_permission_service::delete_role(rid)).await
}

pub async fn get_role(
    ctx: &mut Context<'_>,
    rid: u64,
) -> Result<Message<Role>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetRole { rid };
    Message::from_service(ctx, role_permission_service::get_role(rid)).await
}

pub async fn get_roles_by_user(
    ctx: &mut Context<'_>,
    uid: u64,
) -> Result<Message<Vec<Role>>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetRolesByUser { uid };
    Message::from_service(ctx, role_permission_service::get_roles_by_user(uid)).await
}

pub async fn add_permission(
    ctx: &mut Context<'_>,
    payload: PermissionAddRequest,
) -> Result<Message<()>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::AddPermission;
    Message::from_service(ctx, role_permission_service::add_permission(payload)).await
}

pub async fn delete_permission(
    ctx: &mut Context<'_>,
    pid: u64,
) -> Result<Message<()>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::DeletePermission { pid };
    Message::from_service(ctx, role_permission_service::delete_permission(pid)).await
}

pub async fn get_permission(
    ctx: &mut Context<'_>,
    pid: u64,
) -> Result<Message<Permission>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetPermission { pid };
    Message::from_service(ctx, role_permission_service::get_permission(pid)).await
}

pub async fn get_permissions_by_role(
    ctx: &mut Context<'_>,
    rid: u64,
) -> Result<Message<Vec<Permission>>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetPermissionsByRole { rid };
    Message::from_service(ctx, role_permission_service::get_permissions_by_role(rid)).await
}

pub async fn get_permissions_by_user(
    ctx: &mut Context<'_>,
    uid: u64,
) -> Result<Message<Vec<Permission>>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetPermissionsByUser { uid };
    Message::from_service(ctx, role_permission_service::get_permissions_by_user(uid)).await
}

pub async fn add_permission_for_role(
    ctx: &mut Context<'_>,
    pid: u64,
    rid: u64,
) -> Result<Message<()>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::AddPermissionForRole { pid, rid };
    Message::from_service(
        ctx,
        role_permission_service::add_permission_for_role(pid, rid),
    )
    .await
}

pub async fn add_role_for_user(
    ctx: &mut Context<'_>,
    rid: u64,
    uid: u64,
) -> Result<Message<()>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::AddRoleForUser { rid, uid };
    Message::from_service(ctx, role_permission_service::add_role_for_user(rid, uid)).await
}

pub async fn query_role(ctx: &mut Context<'_>) -> Result<Message<Vec<Role>>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryRole;
    Message::from_service(ctx, role_permission_service::query_role()).await
}

pub async fn query_permission(
    ctx: &mut Context<'_>,
) -> Result<Message<Vec<Permission>>, ApiError<AclError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryPermission;
    Message::from_service(ctx, role_permission_service::query_permission()).await
}
