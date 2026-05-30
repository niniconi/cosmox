use std::path::PathBuf;

use bytes::Bytes;
use cosmox_backend_data::services::file_service::{self, PushResponse};
use futures_util::StreamExt;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message, MessagePayload},
};
pub use cosmox_backend_data::services::file_service::FileError;

pub async fn push_item_octet_stream<S, E>(
    ctx: &mut Context<'_>,
    payload: S,
) -> Result<Message<PushResponse>, ApiError<FileError>>
where
    S: StreamExt<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Display,
{
    ctx.access_ctx.endpoint = api::Endpoint::ItemPush;
    Message::from_service(ctx, file_service::push_item_octet_stream(payload)).await
}

pub async fn pull_item_by_named_file(
    ctx: &mut Context<'_>,
    pmid: u64,
) -> Result<PathBuf, ApiError<FileError>> {
    ctx.access_ctx.endpoint = api::Endpoint::ItemPull { pmid };
    let msg = Message::from_service(ctx, file_service::pull_item_by_named_file(pmid)).await?;
    match msg.payload {
        Some(MessagePayload::Data(path)) => Ok(path),
        _ => Err(ApiError::Logic(FileError::InternalError(
            "no file data".to_string(),
        ))),
    }
}
