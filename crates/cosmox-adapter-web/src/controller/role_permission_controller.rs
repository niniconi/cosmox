use actix_web::{Responder, get, post, web};
use cosmox_backend_api::{
    Context,
    api::{
        self,
        role_permission::{
            AclError, PermissionAddRequest, PermissionDeleteRequest, RoleAddRequest,
            RoleDeleteRequest, RoleLinkPermissionAddRequest,
        },
    },
    message,
};
use cosmox_macros::actix_web_error;

use crate::into_message;

actix_web_error! {
    AclError {
        NotFoundRole() => {code: 404},
        NotFoundPermission() => {code: 404},
        InternalError() => {code: 500},
    }
}

/// Add role
///
/// Create a new user
#[post("/role/add")]
pub async fn add_role(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<RoleAddRequest>,
) -> impl Responder {
    into_message!(api::role_permission::add_role(&mut ctx.into_inner(), payload.into_inner()).await)
}

/// Delete role
#[post("/role/delete")]
pub async fn delete_role(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<RoleDeleteRequest>,
) -> impl Responder {
    into_message!(api::role_permission::delete_role(&mut ctx.into_inner(), params.rid).await)
}

#[post("/permission/add")]
pub async fn add_permission(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<PermissionAddRequest>,
) -> impl Responder {
    into_message!(
        api::role_permission::add_permission(&mut ctx.into_inner(), payload.into_inner()).await
    )
}

#[post("/permission/delete")]
pub async fn delete_permission(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<PermissionDeleteRequest>,
) -> impl Responder {
    into_message!(api::role_permission::delete_permission(&mut ctx.into_inner(), params.pid).await)
}

#[post("/role/permission/add")]
pub async fn add_permission_for_role(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<RoleLinkPermissionAddRequest>,
) -> impl Responder {
    into_message!(
        api::role_permission::add_permission_for_role(
            &mut ctx.into_inner(),
            payload.pid,
            payload.rid
        )
        .await
    )
}

/// Query all roles
#[get("/query/role")]
pub async fn query_role(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(api::role_permission::query_role(&mut ctx.into_inner()).await)
}

/// Query all permissions
#[get("/query/permission")]
pub async fn query_permission(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(api::role_permission::query_permission(&mut ctx.into_inner()).await)
}
