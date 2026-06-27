use std::sync::Arc;

use actix_web::web::Payload;
use actix_web::{Responder, get, post, web};
use cosmox_backend_api::api::role_permission::UserRoleAddRequest;
use cosmox_backend_api::api::user::{
    UserDeleteRequest, UserError, UserLoginRequest, UserQueryRequest, UserSignUpRequest,
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
/// Login by username or email
#[post("/login")]
pub async fn login(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<UserLoginRequest>,
) -> impl Responder {
    into_message!(api::user::login(&mut ctx.into_inner(), Arc::new(payload.into_inner())).await)
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
