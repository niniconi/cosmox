use std::sync::{Arc, Mutex};

use cosmox_api::metadata::Metadata;
use cosmox_backend_data::services::metadata_service;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};

pub use cosmox_backend_data::services::metadata_service::{MetadataError, MetadataQueryRequest};

pub async fn get(ctx: &mut Context<'_>, rid: u64) -> Result<(), MetadataError> {
    ctx.access_ctx.endpoint = api::Endpoint::GetMetadata { rid };
    unimplemented!("Not implemented add api")
}

/// Query metadata from server
pub async fn query(
    ctx: &mut Context<'_>,
    payload: Arc<MetadataQueryRequest>,
) -> Result<Message<Arc<Mutex<Metadata<()>>>>, ApiError<MetadataError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryMetadata;
    Message::from_service(ctx, metadata_service::query_metadata(payload)).await
}
