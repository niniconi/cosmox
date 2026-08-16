use std::sync::Arc;

use actix_web::web::Payload;
use actix_web::{Responder, get, post, web};
use cosmox_backend_api::api::role_permission::UserRoleAddRequest;
use cosmox_backend_api::api::user::{
    DeviceLoginInfo, DeviceQueryRequest, UserDeleteRequest, UserError, UserLoginRequest,
    UserQueryRequest, UserSignUpRequest,
};
use cosmox_backend_api::message::{self};
use cosmox_backend_api::{Context, api};
use cosmox_macros::actix_web_error;

use crate::into_message;

actix_web_error! {
    UserError {
        NotFound() => {code: 404},
        Unauthorized() => {code: 403},
        IdentTaken() => {code: 409},
        InvalidUsernamePassword => {code: 401},
        Validation() => {code: 409},
        AccountLocked() => {code: 403},
        EmailAlreadyRegistered() => {code: 409},
        ConfirmationPasswordMismatch => {code: 409},
        UserCreationFailed() => {code: 500},
        LoginFailed() => {code: 403},
        InternalError() => {code: 500},
    }
}

/// Sign up
///
/// Create a new user
#[post("/register")]
pub async fn register(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<UserSignUpRequest>,
) -> impl Responder {
    into_message!(api::user::register(&mut ctx.into_inner(), Arc::new(payload.into_inner())).await)
}

/// Delete user
#[post("/delete")]
pub async fn delete(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<UserDeleteRequest>,
) -> impl Responder {
    into_message!(api::user::delete(&mut ctx.into_inner(), params.uid).await)
}

/// Query User
#[get("/query")]
pub async fn query(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<UserQueryRequest>,
) -> impl Responder {
    into_message!(api::user::query(&mut ctx.into_inner(), Arc::new(params.into_inner())).await)
}

/// get user
///
/// get user entity by uid
#[get("/{uid}")]
pub async fn get(ctx: web::ReqData<Context<'_>>, uid: web::Path<u64>) -> impl Responder {
    into_message!(api::user::get_user(&mut ctx.into_inner(), *uid).await)
}

/// Login
///
/// Login by username or email, binding the device session to the issuing
/// token so logout can revoke it. The client identity comes from the
/// request metadata: peer IP and User-Agent header.
#[post("/login")]
pub async fn login(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<UserLoginRequest>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let ip = req
        .connection_info()
        .realip_remote_addr()
        .map(|x| x.to_string())
        .or_else(|| req.peer_addr().map(|addr| addr.ip().to_string()))
        .unwrap_or_default();
    let user_agent = req
        .headers()
        .get(actix_web::http::header::USER_AGENT)
        .and_then(|x| x.to_str().ok())
        .map(|x| x.to_string());
    let device_info = DeviceLoginInfo { user_agent, ip };

    into_message!(
        api::user::login(
            &mut ctx.into_inner(),
            Arc::new(payload.into_inner()),
            device_info,
        )
        .await
    )
}

/// List device sessions by filter (admin/audit view).
#[get("/devices")]
pub async fn query_devices(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<DeviceQueryRequest>,
) -> impl Responder {
    into_message!(
        api::user::query_devices(&mut ctx.into_inner(), Arc::new(params.into_inner())).await
    )
}

/// Log out one specific device session: own devices are self-service,
/// other users' devices require the `User.SessionManage` permission.
#[post("/devices/{did}/logout")]
pub async fn logout_device(ctx: web::ReqData<Context<'_>>, did: web::Path<u64>) -> impl Responder {
    into_message!(api::user::logout_device(&mut ctx.into_inner(), *did).await)
}

/// Log out all device sessions of a user: pass your own uid to sign out
/// everywhere; manage another user's sessions requires `User.SessionManage`.
#[post("/{uid}/logout")]
pub async fn logout_user(ctx: web::ReqData<Context<'_>>, uid: web::Path<u64>) -> impl Responder {
    into_message!(api::user::logout_user(&mut ctx.into_inner(), *uid).await)
}

/// upload avatar
///
/// upload a small picture as your account's avatar
#[post("/{uid}/avatar/upload")]
pub async fn upload_avatar(
    ctx: web::ReqData<Context<'_>>,
    uid: web::Path<u64>,
    payload: Payload,
) -> impl Responder {
    into_message!(api::user::upload_user_avatar(&mut ctx.into_inner(), *uid, payload).await)
}

#[post("/role/add")]
pub async fn add_role(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<UserRoleAddRequest>,
) -> impl Responder {
    into_message!(
        api::role_permission::add_role_for_user(&mut ctx.into_inner(), payload.rid, payload.uid,)
            .await
    )
}
