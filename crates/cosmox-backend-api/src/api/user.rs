use std::sync::Arc;

use bytes::Bytes;
use cosmox_backend_data::services::{
    device_service,
    file_service::{FileError, PushResponse},
    user_service::{self, UserResp},
};
pub use cosmox_backend_data::{
    define::User,
    services::{
        device_service::{DeviceLoginInfo, DeviceQueryRequest, DeviceSession},
        user_service::{
            UserDeleteRequest, UserError, UserLoginRequest, UserQueryRequest, UserSignUpRequest,
        },
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
    device_info: DeviceLoginInfo,
) -> Result<Message<String>, ApiError<UserError>> {
    ctx.access_ctx.endpoint = api::Endpoint::Login;
    Message::from_service(ctx, user_service::login(payload, device_info)).await
}

/// List device sessions filtered by `params` (admin/audit view; gated by
/// the `User.Audit` permission in the access check).
pub async fn query_devices(
    ctx: &mut Context<'_>,
    params: Arc<DeviceQueryRequest>,
) -> Result<Message<Vec<DeviceSession>>, ApiError<UserError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryDevices { uid: params.uid };
    Message::<Vec<DeviceSession>>::from_service(ctx, user_service::query_devices(params)).await
}

/// Log out one specific device session.
///
/// The owning user of `did` is resolved here and used as the deletion
/// bound; the permission check has already verified that the caller is the
/// owner (self-service) or holds `User.SessionManage` (admin).
pub async fn logout_device<'a>(
    ctx: &'a mut Context<'a>,
    did: u64,
) -> Result<Message<()>, ApiError<UserError>> {
    ctx.access_ctx.endpoint = api::Endpoint::LogoutDevice { did };
    Message::from_service_with_ctx(ctx, |_ctx| async move {
        match device_service::query_device_owner(did).await {
            Ok(Some(owner)) => user_service::logout_device(owner, did).await,
            Ok(None) => Ok(()),
            Err(err) => Err(UserError::InternalError(format!(
                "Resolve device owner failed: {err}"
            ))),
        }
    })
    .await
}

/// Log out all sessions of `uid`; self-service for the caller's own uid,
/// admin operation (needs `User.SessionManage`) for other users.
pub async fn logout_user(
    ctx: &mut Context<'_>,
    uid: u64,
) -> Result<Message<()>, ApiError<UserError>> {
    ctx.access_ctx.endpoint = api::Endpoint::LogoutUser { uid };
    Message::from_service(ctx, user_service::logout_all(uid)).await
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
