use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use common::message::Pagination;
use cosmox_macros::page_helper;
use futures_util::future::try_join_all;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder, TransactionTrait, TryInsertResult,
};
use serde::{Deserialize, Serialize};

use crate::{
    define::Type,
    entities::{libraries, libraries_related_tags, library_paths, types},
    get_db_connection,
};

/// Errors related to library (collection of media files) operations.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("Library '{0}' not found.")]
    NotFound(u64),

    #[error("User '{0}' does not have access to library '{1}'.")]
    Unauthorized(u64, u64),

    #[error("Library with name '{0}' already exists for user '{1}'.")]
    NameConflict(String, u64),

    #[error("Library '{0}' is empty, cannot perform this operation.")]
    EmptyLibrary(u64),

    #[error("Failed to update library metadata for '{0}': {1}")]
    MetadataUpdateFailed(u64, String),

    #[error("Library '{0}' cannot be deleted due to associated resources.")]
    DeletionConflict(u64),

    #[error("Exceeded maximum number of libraries allowed for user '{0}'.")]
    MaxLibrariesExceeded(u64),

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

#[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct LibraryAddRequest {
    pub name: String,
    pub description: Option<String>,

    pub r#type: u64,

    pub tags: Vec<u64>,
    pub library_paths: Vec<String>,
}

#[page_helper]
#[derive(Debug, Deserialize)]
pub struct LibraryQueryRequest {}

#[derive(Debug, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct ModifyLibraryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// create library with tags and paths
/// # Arguments
/// - `payload` - library creation request
/// - `uid` - user who creates the library
pub async fn create_library_with_tags_and_paths(
    payload: Arc<LibraryAddRequest>,
    uid: u64,
) -> Result<
    (
        libraries::Model,
        Vec<libraries_related_tags::Model>,
        Vec<library_paths::Model>,
    ),
    LibraryError,
> {
    let db = get_db_connection().await;
    // check

    // insert into database
    let result = db
        .clone()
        .transaction::<_, (libraries::Model, Vec<_>, Vec<_>), LibraryError>(|_txn| {
            Box::pin(async move {
                let current_datetime = Utc::now().naive_utc();
                let library = libraries::ActiveModel {
                    name: Set(Some(payload.name.clone())),
                    description: Set(payload.description.clone()),
                    create_datetime: Set(current_datetime),
                    last_update_datetime: Set(current_datetime),
                    create_by_uid: Set(uid),
                    ..Default::default()
                };
                let library = library
                    .insert(db.as_ref())
                    .await
                    .inspect_err(|err| log::error!("{err}"))
                    .map_err(|err| {
                        LibraryError::InternalError(format!("Insert library failed: {err}"))
                    })?;

                let add_tag_futures: Vec<_> = payload
                    .tags
                    .iter()
                    .map(|x| async {
                        let library_tag_relation = libraries_related_tags::ActiveModel {
                            lid: Set(library.lid),
                            tid: Set(*x),
                            ..Default::default()
                        };
                        library_tag_relation
                            .insert(db.as_ref())
                            .await
                            .inspect_err(|err| log::error!("{err}"))
                            .map_err(|err| {
                                LibraryError::InternalError(format!(
                                    "Insert tag for library failed: {err}"
                                ))
                            })
                    })
                    .collect();

                let add_tag_results = try_join_all(add_tag_futures).await?;

                let add_path_futures: Vec<_> = payload
                    .library_paths
                    .iter()
                    .map(|path| async {
                        let library_path = library_paths::ActiveModel {
                            lid: Set(library.lid),
                            path: Set(path.clone()),
                            ..Default::default()
                        };
                        library_path
                            .insert(db.as_ref())
                            .await
                            .inspect_err(|err| log::error!("{err}"))
                            .map_err(|err| {
                                LibraryError::InternalError(format!(
                                    "Insert path for library failed: {err}"
                                ))
                            })
                    })
                    .collect();

                let add_path_results = try_join_all(add_path_futures).await?;
                Ok((library, add_tag_results, add_path_results))
            })
        })
        .await;

    let result = result.map_err(|err| {
        log::error!("Transaction failed when adding library: {err}");
        LibraryError::InternalError(format!("Transaction failed: {err}"))
    })?;
    Ok(result)
}

/// delete library
pub async fn delete_library(lid: u64) -> Result<(), LibraryError> {
    let db = get_db_connection().await;
    libraries::Entity::delete_by_id(lid)
        .exec(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            LibraryError::InternalError(format!("Delete library {lid} failed: {err}"))
        })?;
    Ok(())
}

pub async fn add_tags_for_library(
    lid: u64,
    tags: Vec<u64>,
) -> Result<Vec<libraries_related_tags::Model>, LibraryError> {
    let db = get_db_connection().await;
    let add_tag_futures: Vec<_> = tags
        .iter()
        .map(|x| async {
            let library_tag_relation = libraries_related_tags::ActiveModel {
                lid: Set(lid),
                tid: Set(*x),
                ..Default::default()
            };
            library_tag_relation
                .insert(db.as_ref())
                .await
                .inspect_err(|err| log::error!("{err}"))
        })
        .collect();

    let add_tags_result = try_join_all(add_tag_futures).await.map_err(|err| {
        LibraryError::InternalError(format!("Add tags for library {lid} failed: {err}"))
    })?;
    Ok(add_tags_result)
}

pub async fn get_library(lid: u64) -> Result<libraries::Model, LibraryError> {
    let db = get_db_connection().await;
    let library = libraries::Entity::find_by_id(lid)
        .one(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| LibraryError::InternalError(format!("Get library {lid} failed: {err}")))?;

    match library {
        Some(library) => Ok(library),
        None => Err(LibraryError::NotFound(lid)),
    }
}

pub async fn query_libraries(
    params: LibraryQueryRequest,
) -> Result<(Vec<libraries::Model>, Pagination), LibraryError> {
    let db = get_db_connection().await;
    let mut select = libraries::Entity::find();
    let mut page = 0;

    if let Some(inner_page) = params.page {
        page = inner_page;
    }

    if let Some(sort) = &params.sort
        && let Ok(column) = libraries::Column::from_str(sort)
    {
        select = select.order_by(column, sea_orm::Order::Asc);
    };

    let paginator = select.paginate(db.as_ref(), params.page_size);
    let total = paginator
        .num_items()
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| LibraryError::InternalError(format!("Count libraries failed: {err}")))?;
    let pagination = Pagination::new(total, params.page_size, paginator.cur_page(), "");

    match paginator.fetch_page(page).await {
        Ok(result) => Ok((result, pagination)),
        Err(err) => {
            log::error!("{err}");
            Err(LibraryError::InternalError(format!(
                "Query libraries failed: {err}"
            )))
        }
    }
}

pub async fn check_if_paths_overlap(paths: Vec<String>, _db: Arc<DatabaseConnection>) -> bool {
    true
}

pub async fn get_all_type() -> Result<Vec<Type>, LibraryError> {
    let db = get_db_connection().await;
    types::Entity::find()
        .all(db.as_ref())
        .await
        .map_err(|err| LibraryError::InternalError(format!("Get all types failed: {err}")))
}

/// Insert media types into the `types` table, skipping duplicates.
pub async fn add_media_types(media_types: Vec<String>) -> Result<(), LibraryError> {
    let db = get_db_connection().await;

    let models = media_types.iter().map(|ty| types::ActiveModel {
        label: Set(ty.clone()),
        ..Default::default()
    });

    let result = types::Entity::insert_many(models)
        .on_conflict_do_nothing()
        .exec(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| LibraryError::InternalError(format!("Insert media types failed: {err}")))?;

    match &result {
        TryInsertResult::Empty => {
            log::warn!("Skip insertion: No data provided for types {media_types:?}.")
        }
        TryInsertResult::Inserted(_) => {
            log::debug!("Successfully inserted types to database: {media_types:?}.")
        }
        TryInsertResult::Conflicted => {
            log::debug!("Some media types already exist: {media_types:?}.")
        }
    }

    log::info!("Add media types {media_types:?} successful.");
    Ok(())
}

pub async fn modify_library(lid: u64, payload: ModifyLibraryRequest) -> Result<(), LibraryError> {
    let db = get_db_connection().await;
    let current_datetime = Utc::now().naive_utc();

    let name = match payload.name {
        Some(name) => Set(Some(name)),
        None => NotSet,
    };
    let description = match payload.description {
        Some(description) => Set(Some(description)),
        None => NotSet,
    };
    let last_update_datetime = Set(current_datetime);

    let library = libraries::ActiveModel {
        lid: Set(lid),
        name,
        description,
        last_update_datetime,
        ..Default::default()
    };

    libraries::Entity::update(library)
        .exec(db.as_ref())
        .await
        .map_err(|err| {
            LibraryError::InternalError(format!("Modify library {lid} failed: {err}"))
        })?;

    Ok(())
}
