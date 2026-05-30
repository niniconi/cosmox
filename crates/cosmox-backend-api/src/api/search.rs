use cosmox_backend_data::services::search_service;

use crate::{
    Context,
    message::{ApiError, FromService, Message},
};

pub use cosmox_backend_data::{
    define::Resource,
    services::search_service::{SearchError, SearchRequest},
};

pub async fn search(
    ctx: &mut Context<'_>,
    payload: SearchRequest,
) -> Result<Message<Vec<Resource>>, ApiError<SearchError>> {
    ctx.access_ctx.endpoint = crate::api::Endpoint::Search;
    Message::<Vec<Resource>>::from_service(ctx, search_service::search(payload)).await
}
