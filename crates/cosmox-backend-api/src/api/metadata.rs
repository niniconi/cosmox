use std::sync::Arc;

use cosmox_api::metadata::MetadataNode;
use cosmox_backend_data::services::metadata_service;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};

pub use cosmox_backend_data::services::metadata_service::{
    MetadataError, MetadataQueryKey, MetadataQueryRequest,
};

pub async fn get(
    ctx: &mut Context<'_>,
    rid: u64,
) -> Result<Message<MetadataNode>, ApiError<MetadataError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetMetadata { rid };
    let payload = Arc::new(MetadataQueryRequest {
        root: MetadataQueryKey::Id(rid),
        depth: 1,
    });
    Message::from_service(ctx, metadata_service::query_metadata(payload)).await
}

/// Query metadata from server
pub async fn query(
    ctx: &mut Context<'_>,
    key: MetadataQueryKey,
    depth: usize,
) -> Result<Message<MetadataNode>, ApiError<MetadataError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryMetadata;
    let payload = Arc::new(MetadataQueryRequest { root: key, depth });
    Message::from_service(ctx, metadata_service::query_metadata(payload)).await
}
