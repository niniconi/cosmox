use std::convert::Infallible;

use cosmox_backend_data::services::system_service::{self, SystemInfo};

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};
pub use cosmox_backend_data::services::system_service::SystemError;

pub async fn info(ctx: &mut Context<'_>) -> Result<Message<SystemInfo>, ApiError<SystemError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetSystemInfo;
    Message::from_service(ctx, system_service::info()).await
}

/// Restart server
pub async fn restart(ctx: &mut Context<'_>) -> Result<(), ApiError<SystemError>> {
    ctx.access_ctx.endpoint = api::Endpoint::SystemRestart;
    unimplemented!("Not implemented restart api")
}

pub async fn shutdown(ctx: &mut Context<'_>) -> Result<Message<()>, ApiError<Infallible>> {
    ctx.access_ctx.endpoint = api::Endpoint::SystemShutdown;
    Message::from_service(ctx, async {
        system_service::shutdown().await;
        #[allow(unreachable_code)]
        Ok::<(), Infallible>(())
    })
    .await
}

pub async fn about(ctx: &mut Context<'_>) -> Result<Message<String>, ApiError<SystemError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetSystemAbout;
    Message::from_service(ctx, system_service::about()).await
}

pub async fn log(ctx: &mut Context<'_>) -> Result<Message<String>, ApiError<SystemError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetSystemLog;
    Message::from_service(ctx, system_service::log()).await
}

pub async fn delete_all(ctx: &mut Context<'_>) -> Result<Message<()>, ApiError<SystemError>> {
    ctx.access_ctx.endpoint = api::Endpoint::SystemDeleteAll;
    Message::from_service(ctx, system_service::delete_all()).await
}
