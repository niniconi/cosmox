use cosmox_backend_data::{
    define::{Tag, TagGroups},
    services::tag_service::{self},
};

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};

pub use cosmox_backend_data::services::tag_service::{
    TagAddRequest, TagError, TagGroupAddRequest, TagGroupDeleteRequest, TagGroupQueryRequest,
    TagQueryRequest,
};

pub async fn get(ctx: &mut Context<'_>, tid: u64) -> Result<Message<Tag>, ApiError<TagError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetTag { tid };
    Message::from_service(ctx, tag_service::get_tag(tid)).await
}

pub async fn get_group(
    ctx: &mut Context<'_>,
    tgid: u64,
) -> Result<Message<TagGroups>, ApiError<TagError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetTagGroup { tgid };
    Message::from_service(ctx, tag_service::get_tag_group(tgid)).await
}

pub async fn add(
    ctx: &mut Context<'_>,
    payload: TagAddRequest,
) -> Result<Message<u64>, ApiError<TagError>> {
    ctx.access_ctx.endpoint = api::Endpoint::AddTag;
    Message::from_service(ctx, tag_service::add_tag(payload.label, payload.tgid)).await
}

pub async fn add_group(
    ctx: &mut Context<'_>,
    payload: TagGroupAddRequest,
) -> Result<Message<u64>, ApiError<TagError>> {
    ctx.access_ctx.endpoint = api::Endpoint::AddTagGroup;
    Message::from_service(ctx, tag_service::add_tag_group(payload.label)).await
}

pub async fn delete_group(
    ctx: &mut Context<'_>,
    payload: TagGroupDeleteRequest,
) -> Result<Message<()>, ApiError<TagError>> {
    ctx.access_ctx.endpoint = api::Endpoint::DeleteTagGroup { tgid: payload.tgid };
    Message::from_service(ctx, tag_service::delete_tag_group(payload.tgid)).await
}

pub async fn query(
    ctx: &mut Context<'_>,
    payload: TagQueryRequest,
) -> Result<Message<Vec<Tag>>, ApiError<TagError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryTag;
    Message::<Vec<Tag>>::from_service(ctx, tag_service::query_tag(payload)).await
}

pub async fn query_group(
    ctx: &mut Context<'_>,
    payload: TagGroupQueryRequest,
) -> Result<Message<Vec<TagGroups>>, ApiError<TagError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryTagGroup;
    Message::<Vec<TagGroups>>::from_service(ctx, tag_service::query_tag_group(payload)).await
}

pub async fn catalog(
    ctx: &mut Context<'_>,
) -> Result<Message<Vec<tag_service::TagCatalogEntry>>, ApiError<TagError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetTagCatalog;
    Message::from_service(ctx, tag_service::query_catalog()).await
}
