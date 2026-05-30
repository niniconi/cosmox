use cosmox_scanner::scanner_manager;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};

pub use cosmox_backend_data::services::scanner_service::{
    ScannerError, ScannerInfo, ScannerStatus, SelectedLibraries,
};

/// Scan library by id
pub async fn scan(
    ctx: &mut Context<'_>,
    lid: u64,
) -> Result<Message<&'static str>, ApiError<ScannerError>> {
    ctx.access_ctx.endpoint = api::Endpoint::Scan { lid };
    Message::from_service(ctx, async move {
        scanner_manager::start(scanner_manager::SelectedLibraries::SINGLE(lid)).await?;
        Ok("complete")
    })
    .await
}

/// Scan all libraries
pub async fn scan_all(
    ctx: &mut Context<'_>,
) -> Result<Message<&'static str>, ApiError<ScannerError>> {
    ctx.access_ctx.endpoint = api::Endpoint::ScanAll;
    Message::from_service(ctx, async move {
        scanner_manager::start(scanner_manager::SelectedLibraries::ALL).await?;
        Ok("complete")
    })
    .await
}

pub async fn add_task(ctx: &mut Context<'_>) -> Result<(), ScannerError> {
    ctx.access_ctx.endpoint = api::Endpoint::AddScanTask;
    unimplemented!("Add task api is not yet implemented.")
}

/// get status of scanner
///
pub async fn processed(
    ctx: &mut Context<'_>,
) -> Result<Message<ScannerStatus>, ApiError<ScannerError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetProcessOfScan;
    Message::from_service(
        ctx,
        cosmox_backend_data::services::scanner_service::get_scanner_status(),
    )
    .await
}

/// get information of scanner
///
pub async fn info(ctx: &mut Context<'_>) -> Result<Message<ScannerInfo>, ApiError<ScannerError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetScannerInfo;
    Message::from_service(
        ctx,
        cosmox_backend_data::services::scanner_service::get_scanner_info(),
    )
    .await
}
