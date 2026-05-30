use std::sync::Arc;

use bytes::Bytes;
use cosmox_backend_data::services::{
    file_service::{FileError, PushResponse},
    user_service::{self, UserResp},
};
pub use cosmox_backend_data::{
    define::User,
    services::user_service::{
        UserDeleteRequest, UserError, UserLoginRequest, UserQueryRequest, UserSignUpRequest,
    },
};
use futures_util::StreamExt;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};

pub async fn get_user(
    ctx: &mut Context<'_>,
    uid: u64,
) -> Result<Message<User>, ApiError<UserError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetUser { uid };
    Message::from_service(ctx, user_service::get_user(uid)).await
}

pub async fn register(
    ctx: &mut Context<'_>,
    payload: Arc<UserSignUpRequest>,
) -> Result<Message<UserResp>, ApiError<UserError>> {
    ctx.access_ctx.endpoint = api::Endpoint::Register;
    Message::from_service(ctx, user_service::sign_up(payload)).await
}

pub async fn login(
    ctx: &mut Context<'_>,
    payload: Arc<UserLoginRequest>,
) -> Result<Message<String>, ApiError<UserError>> {
    ctx.access_ctx.endpoint = api::Endpoint::Login;
    Message::from_service(ctx, user_service::login(payload)).await
}

pub async fn delete(ctx: &mut Context<'_>, uid: u64) -> Result<Message<()>, ApiError<UserError>> {
    ctx.access_ctx.endpoint = api::Endpoint::DeleteUser { uid };
    Message::from_service(ctx, user_service::delete(uid)).await
}

pub async fn query(
    ctx: &mut Context<'_>,
    payload: Arc<UserQueryRequest>,
) -> Result<Message<Vec<User>>, ApiError<UserError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryUser;
    Message::<Vec<User>>::from_service(ctx, user_service::query(payload)).await
}

pub async fn upload_user_avatar<S, E>(
    ctx: &mut Context<'_>,
    uid: u64,
    payload: S,
) -> Result<Message<PushResponse>, ApiError<FileError>>
where
    S: StreamExt<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Display,
{
    ctx.access_ctx.endpoint = api::Endpoint::UploadAvatar { uid };
    Message::from_service(ctx, user_service::upload_user_avatar(uid, payload)).await
}
