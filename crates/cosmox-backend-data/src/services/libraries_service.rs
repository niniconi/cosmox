use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

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

    #[error("Library paths overlap: {0}")]
    PathOverlap(String),

    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("Permission denied for path: {0}")]
    PathPermissionDenied(String),

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
    create_library_with_tags_and_paths_db(&db, payload, uid).await
}

pub async fn create_library_with_tags_and_paths_db(
    db: &DatabaseConnection,
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
    let overlapping_paths = find_overlapping_paths(&payload.library_paths)?;
    if !overlapping_paths.is_empty() {
        let overlapping_paths = overlapping_paths.join(",");
        log::error!("Failed to create library: contains overlapping paths '{overlapping_paths}'");
        return Err(LibraryError::PathOverlap(overlapping_paths));
    }

    // insert into database
    let result = db
        .clone()
        .transaction::<_, (libraries::Model, Vec<_>, Vec<_>), LibraryError>(|txn| {
            Box::pin(async move {
                let current_datetime = Utc::now().naive_utc();
                let library = libraries::ActiveModel {
                    name: Set(Some(payload.name.clone())),
                    description: Set(payload.description.clone()),
                    create_datetime: Set(current_datetime),
                    last_update_datetime: Set(current_datetime),
                    create_by_uid: Set(uid),
                    r#type: Set(payload.r#type),
                    ..Default::default()
                };
                let library = library
                    .insert(txn)
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
                            .insert(txn)
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
                            .insert(txn)
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
    delete_library_db(&db, lid).await
}

pub async fn delete_library_db(db: &DatabaseConnection, lid: u64) -> Result<(), LibraryError> {
    libraries::Entity::delete_by_id(lid)
        .exec(db)
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
    add_tags_for_library_db(&db, lid, tags).await
}

pub async fn add_tags_for_library_db(
    db: &DatabaseConnection,
    lid: u64,
    tags: Vec<u64>,
) -> Result<Vec<libraries_related_tags::Model>, LibraryError> {
    let add_tag_futures: Vec<_> = tags
        .iter()
        .map(|x| async {
            let library_tag_relation = libraries_related_tags::ActiveModel {
                lid: Set(lid),
                tid: Set(*x),
                ..Default::default()
            };
            library_tag_relation
                .insert(db)
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
    get_library_db(&db, lid).await
}

pub async fn get_library_db(
    db: &DatabaseConnection,
    lid: u64,
) -> Result<libraries::Model, LibraryError> {
    let library = libraries::Entity::find_by_id(lid)
        .one(db)
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
    query_libraries_db(&db, params).await
}

pub async fn query_libraries_db(
    db: &DatabaseConnection,
    params: LibraryQueryRequest,
) -> Result<(Vec<libraries::Model>, Pagination), LibraryError> {
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

    let paginator = select.paginate(db, params.page_size);
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

/// Returns the overlapping paths, if any.
fn find_overlapping_paths(paths: &[String]) -> Result<Vec<String>, LibraryError> {
    let mut sorted_paths: Vec<PathBuf> = vec![];
    for path in paths {
        sorted_paths.push(Path::new(path).canonicalize().map_err(|err| {
            log::error!("Failed to canonicalize path '{path}': {err}");
            match err.kind() {
                std::io::ErrorKind::NotFound => LibraryError::PathNotFound(err.to_string()),
                std::io::ErrorKind::PermissionDenied => {
                    LibraryError::PathPermissionDenied(err.to_string())
                }
                _ => LibraryError::InternalError(err.to_string()),
            }
        })?);
    }
    // Sorting is key: parent paths will always come before child paths
    sorted_paths.sort();

    let mut overlapping = Vec::new();

    for i in 0..sorted_paths.len() {
        for j in i + 1..sorted_paths.len() {
            if sorted_paths[j].starts_with(&sorted_paths[i]) {
                overlapping.push(sorted_paths[j].to_string_lossy().into_owned());
            } else {
                break;
            }
        }
    }

    // Remove duplicates to prevent a path from being included multiple times by multiple parents
    overlapping.sort();
    overlapping.dedup();

    Ok(overlapping)
}

pub async fn get_all_type() -> Result<Vec<Type>, LibraryError> {
    let db = get_db_connection().await;
    get_all_type_db(&db).await
}

pub async fn get_all_type_db(db: &DatabaseConnection) -> Result<Vec<Type>, LibraryError> {
    types::Entity::find()
        .all(db)
        .await
        .map_err(|err| LibraryError::InternalError(format!("Get all types failed: {err}")))
}

/// Insert media types into the `types` table, skipping duplicates.
pub async fn add_media_types(media_types: Vec<String>) -> Result<(), LibraryError> {
    let db = get_db_connection().await;
    add_media_types_db(&db, media_types).await
}

pub async fn add_media_types_db(
    db: &DatabaseConnection,
    media_types: Vec<String>,
) -> Result<(), LibraryError> {
    let models = media_types.iter().map(|ty| types::ActiveModel {
        label: Set(ty.clone()),
        ..Default::default()
    });

    let result = types::Entity::insert_many(models)
        .on_conflict_do_nothing()
        .exec(db)
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
    modify_library_db(&db, lid, payload).await
}

pub async fn modify_library_db(
    db: &DatabaseConnection,
    lid: u64,
    payload: ModifyLibraryRequest,
) -> Result<(), LibraryError> {
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
        .exec(db)
        .await
        .map_err(|err| {
            LibraryError::InternalError(format!("Modify library {lid} failed: {err}"))
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Create real directories under `/tmp/cosmox_lib_test/{test_name}/` for testing,
    /// so that `.canonicalize()` succeeds. Each test must pass a unique `test_name`
    /// to avoid interference when tests run in parallel.
    fn create_test_dirs(test_name: &str, subdirs: &[&str]) -> Vec<String> {
        let root = PathBuf::from("/tmp/cosmox_lib_test").join(test_name);
        let _ = fs::remove_dir_all(&root);
        subdirs
            .iter()
            .map(|d| {
                let p = root.join(d);
                fs::create_dir_all(&p).unwrap();
                p.to_string_lossy().into_owned()
            })
            .collect()
    }

    fn remove_test_dirs(test_name: &str) {
        let root = PathBuf::from("/tmp/cosmox_lib_test").join(test_name);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_list_returns_no_overlap() {
        assert!(find_overlapping_paths(&[]).unwrap().is_empty());
    }

    #[test]
    fn single_path_returns_no_overlap() {
        let paths = create_test_dirs("single", &["movies"]);
        assert!(find_overlapping_paths(&paths).unwrap().is_empty());
        remove_test_dirs("single");
    }

    #[test]
    fn independent_paths_no_overlap() {
        let paths = create_test_dirs("independent", &["movies", "tv", "music"]);
        assert!(find_overlapping_paths(&paths).unwrap().is_empty());
        remove_test_dirs("independent");
    }

    #[test]
    fn sibling_subdirs_no_overlap() {
        let paths = create_test_dirs("siblings", &["media/movies", "media/tv", "media/music"]);
        assert!(find_overlapping_paths(&paths).unwrap().is_empty());
        remove_test_dirs("siblings");
    }

    #[test]
    fn parent_child_overlap_returns_child() {
        let paths = create_test_dirs("parent_child", &["media", "media/movies"]);
        let result = find_overlapping_paths(&paths).unwrap();
        assert_eq!(result.len(), 1);
        let expected = paths.iter().find(|p| p.ends_with("media/movies")).unwrap();
        assert_eq!(&result[0], expected);
        remove_test_dirs("parent_child");
    }

    #[test]
    fn multiple_children_all_returned() {
        let paths = create_test_dirs("multi_child", &["media", "media/movies", "media/tv"]);
        let result = find_overlapping_paths(&paths).unwrap();
        assert_eq!(result.len(), 2);
        for p in &result {
            assert!(p.ends_with("media/movies") || p.ends_with("media/tv"));
        }
        remove_test_dirs("multi_child");
    }

    #[test]
    fn deeply_nested_overlap_dedup() {
        // a/b/c is a descendant of both `a` and `a/b` — the dedup logic
        // must ensure it appears only once in the result.
        let paths = create_test_dirs("deep_nested", &["a", "a/b", "a/b/c"]);
        let result = find_overlapping_paths(&paths).unwrap();
        assert_eq!(result.len(), 2);
        for p in &result {
            assert!(p.ends_with("a/b") || p.ends_with("a/b/c"));
        }
        remove_test_dirs("deep_nested");
    }

    #[test]
    fn same_path_twice_reported_as_overlap() {
        let paths = create_test_dirs("same_path", &["dup", "dup"]);
        let result = find_overlapping_paths(&paths).unwrap();
        assert_eq!(result.len(), 1);
        let expected = paths.iter().find(|p| p.ends_with("dup")).unwrap();
        assert_eq!(&result[0], expected);
        remove_test_dirs("same_path");
    }

    #[test]
    fn multiple_parents_all_children_returned() {
        // a/x is child of a, b/y is child of b — both overlaps reported
        let paths = create_test_dirs("multi_parent", &["a", "a/x", "b", "b/y"]);
        let result = find_overlapping_paths(&paths).unwrap();
        assert_eq!(result.len(), 2);
        for p in &result {
            assert!(p.ends_with("a/x") || p.ends_with("b/y"));
        }
        remove_test_dirs("multi_parent");
    }

    #[test]
    fn non_existent_path_returns_error() {
        let result =
            find_overlapping_paths(&["/tmp/cosmox_lib_test__nonexistent__xyz".to_string()]);
        assert!(result.is_err());
        assert!(matches!(result, Err(LibraryError::PathNotFound(_))));
    }

    #[test]
    fn non_existent_path_among_valid_returns_error() {
        let root = PathBuf::from("/tmp/cosmox_lib_test").join("partial_missing");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("existing")).unwrap();
        let paths = vec![
            root.join("existing").to_string_lossy().into_owned(),
            root.join("does_not_exist").to_string_lossy().into_owned(),
        ];
        let result = find_overlapping_paths(&paths);
        assert!(result.is_err());
        assert!(matches!(result, Err(LibraryError::PathNotFound(_))));
        let _ = fs::remove_dir_all(&root);
    }
}
