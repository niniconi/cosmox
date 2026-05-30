use cosmox_backend_data::{
    define::{Resource, ResourcesRelatedTags},
    services::resource_service::{self},
};

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};

pub use cosmox_backend_data::services::resource_service::{
    ResourceAddRequest, ResourceAddTagRequest, ResourceDeleteRequest, ResourceError,
    ResourceQueryRequest,
};

pub async fn get(
    ctx: &mut Context<'_>,
    rid: u64,
) -> Result<Message<Resource>, ApiError<ResourceError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetResource { rid };
    Message::from_service(ctx, resource_service::get_resource(rid)).await
}

pub async fn add(
    ctx: &mut Context<'_>,
    payload: ResourceAddRequest,
) -> Result<Message<u64>, ApiError<ResourceError>> {
    ctx.access_ctx.endpoint = api::Endpoint::AddResource;
    Message::from_service(ctx, resource_service::add_resource(payload)).await
}

pub async fn delete(
    ctx: &mut Context<'_>,
    payload: ResourceDeleteRequest,
) -> Result<Message<()>, ApiError<ResourceError>> {
    ctx.access_ctx.endpoint = api::Endpoint::DeleteResource { rid: payload.rid };
    Message::from_service(ctx, resource_service::delete_resource(payload.rid)).await
}

pub async fn query(
    ctx: &mut Context<'_>,
    payload: ResourceQueryRequest,
) -> Result<Message<Vec<Resource>>, ApiError<ResourceError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryResource;
    Message::<Vec<Resource>>::from_service(ctx, resource_service::query_resources(payload)).await
}

pub async fn get_metadata(ctx: &mut Context<'_>, rid: u64) -> Result<(), ResourceError> {
    ctx.access_ctx.endpoint = api::Endpoint::GetMetadataOfResource { rid };
    unimplemented!("Not implemented {{id}}/metadata api")
}

pub async fn add_tag(
    ctx: &mut Context<'_>,
    rid: u64,
    payload: ResourceAddTagRequest,
) -> Result<Message<Vec<ResourcesRelatedTags>>, ApiError<ResourceError>> {
    ctx.access_ctx.endpoint = api::Endpoint::AddTagForResource { rid };
    Message::from_service(
        ctx,
        resource_service::add_tags_for_resource(rid, payload.tags.clone()),
    )
    .await
}
