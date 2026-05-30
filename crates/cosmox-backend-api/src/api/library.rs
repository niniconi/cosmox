use std::sync::Arc;

use cosmox_backend_data::services::libraries_service;

use crate::{
    Context, api,
    message::{ApiError, FromService, Message},
};

pub use cosmox_backend_data::{
    define::{LibrariesRelatedTags, Library, LibraryPath, Type},
    services::libraries_service::{
        LibraryAddRequest, LibraryError, LibraryQueryRequest, ModifyLibraryRequest,
    },
};

pub async fn get(
    ctx: &mut Context<'_>,
    lid: u64,
) -> Result<Message<Library>, ApiError<LibraryError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetLibrary { lid };
    Message::from_service(ctx, libraries_service::get_library(lid)).await
}

pub async fn query(
    ctx: &mut Context<'_>,
    payload: LibraryQueryRequest,
) -> Result<Message<Vec<Library>>, ApiError<LibraryError>> {
    ctx.access_ctx.endpoint = api::Endpoint::QueryLibrary;
    Message::<Vec<Library>>::from_service(ctx, libraries_service::query_libraries(payload)).await
}

pub async fn modify(
    ctx: &mut Context<'_>,
    lid: u64,
    payload: ModifyLibraryRequest,
) -> Result<Message<()>, ApiError<LibraryError>> {
    ctx.access_ctx.endpoint = api::Endpoint::ModifyLibrary { lid };
    Message::from_service(ctx, libraries_service::modify_library(lid, payload)).await
}

/// add library
///
/// add library with tags and path in disk.
pub async fn add<'ctx>(
    ctx: &'ctx mut Context<'ctx>,
    payload: Arc<LibraryAddRequest>,
) -> Result<Message<(Library, Vec<LibrariesRelatedTags>, Vec<LibraryPath>)>, ApiError<LibraryError>>
{
    ctx.access_ctx.endpoint = api::Endpoint::AddLibrary;
    Message::from_service_with_ctx(ctx, |ctx| async move {
        let uid = ctx.request_user.uid.ok_or_else(|| {
            log::error!("Expect `uid` at {:?}", ctx.request_user);
            LibraryError::InternalError("Expected a uid, got None".to_string())
        })?;
        libraries_service::create_library_with_tags_and_paths(payload, uid).await
    })
    .await
}

/// delete library
///
/// delete the entity in database table library.
/// delete the metadata information in disk (Option)
pub async fn delete(
    ctx: &mut Context<'_>,
    lid: u64,
) -> Result<Message<()>, ApiError<LibraryError>> {
    ctx.access_ctx.endpoint = api::Endpoint::DeleteLibrary { lid };
    Message::from_service(ctx, libraries_service::delete_library(lid)).await
}

/// Returns all selectable Types
pub async fn get_all_type(
    ctx: &mut Context<'_>,
) -> Result<Message<Vec<Type>>, ApiError<LibraryError>> {
    ctx.access_ctx.endpoint = api::Endpoint::GetAllLibraryTypes;
    Message::from_service(ctx, libraries_service::get_all_type()).await
}
