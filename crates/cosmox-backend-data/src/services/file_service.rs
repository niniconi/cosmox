use std::path::{Path, PathBuf};

use bytes::Bytes;
use common::fs::FileCleanupGuard;
use common::security::check_new_path_safe;
use cosmox_configuration::Configuration;
use futures_util::StreamExt;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde::Serialize;
use tokio::{
    fs::File,
    io::{self, AsyncWriteExt},
};
use url::Url;

use crate::{
    // io::file_controller::{FileError, PushResponse},
    entities::path_mappings,
    get_db_connection,
};

#[derive(Serialize)]
pub struct PushResponse {
    pub pmid: u64,
    pub uploaded_size: u64,
}

/// Errors related to file operations (upload, download, management).
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("File '{0}' not found.")]
    NotFound(u64),

    #[error("Not authorized to perform file operation on '{0}'.")]
    Unauthorized(String),

    #[error("File '{0}' already exists.")]
    AlreadyExists(String),

    #[error("File upload failed: {0}")]
    UploadFailed(String),

    #[error("File download failed: {0}")]
    DownloadFailed(String),

    #[error("File '{0}' is too large; maximum size is {1} bytes.")]
    TooLarge(String, u64),

    #[error("Invalid file type: {0}")]
    InvalidFileType(String),

    #[error("Not supported scheme: {0}")]
    NotSupportedScheme(String),

    #[error("Not supported Content-Type: {0}")]
    NotSupportedContentType(String),

    #[error("Disk space exhausted")]
    InsufficientStorage,

    #[error("File operation timed out for '{0}'.")]
    // Gateway Timeout if waiting for external storage, or Request Timeout if internal file processing took too long
    OperationTimeout(String),

    #[error("Security block: Malicious relative path detected in tarball: {0}")]
    PathTraversalAttack(String),

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

pub trait ItemFile {
    fn name(&mut self) -> Option<&str>;
}

fn get_default_items_dir() -> Result<PathBuf, FileError> {
    let mut default_path =
        PathBuf::from(&Configuration::get_global_configuration().cosmox.data.path);
    default_path.push("files");
    if !default_path.exists() {
        std::fs::create_dir_all(&default_path)
            .inspect_err(|err| {
                log::error!(
                    "Failed to create upload directory {:?}: {err}",
                    default_path
                );
            })
            .map_err(|err| match err.kind() {
                io::ErrorKind::StorageFull => FileError::InsufficientStorage,
                _ => FileError::InternalError(format!("Failed to create upload directory: {err}")),
            })?;
    }
    Ok(default_path)
}

async fn store_path_mapping<P: AsRef<Path>>(path: P) -> Result<u64, FileError> {
    let path = path.as_ref();
    let canonical_path = path
        .canonicalize()
        .inspect_err(|err| log::error!("Failed to canonicalize uploaded file path: {err}"))
        .map_err(|err| FileError::InternalError(format!("Failed to resolve file path: {err}")))?;
    let path_str = canonical_path.to_str().ok_or_else(|| {
        let err = format!("Uploaded file path is not valid UTF-8: {canonical_path:?}");
        log::error!("{err}");
        FileError::InternalError(err)
    })?;
    let url_string = format!("file://{path_str}");

    let url = Url::parse(url_string.as_str())
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|_err| FileError::InternalError(format!("Parse url error path: {path:?}")))?;

    let pmid = push_item_link(url)
        .await
        .map_err(|err| FileError::InternalError(err.to_string()))?;

    Ok(pmid)
}

async fn local_file_handler(url: &Url) -> Result<PathBuf, FileError> {
    Ok(PathBuf::from(url.path()))
}

async fn http_file_handler(url: &Url, id: u64) -> Result<PathBuf, FileError> {
    let mut resp = reqwest::get(url.as_str())
        .await
        .inspect_err(|err| log::error!("Download file({id}) {url} failed: {err}"))
        .map_err(|_err| FileError::DownloadFailed(url.to_string()))?;

    if !resp.status().is_success() {
        log::error!(
            "Download file({id}) {url} failed: http error code {}",
            resp.status()
        );
        return Err(FileError::DownloadFailed(url.to_string()));
    }

    let temp_dir = std::env::temp_dir();
    let filename = format!("download_{}.tmp", uuid::Uuid::new_v4());
    let temp_path = temp_dir.join(filename);

    let mut file = File::create(&temp_path)
        .await
        .inspect_err(|err| log::error!("Create temp file({temp_path:?}) error:{err}"))
        .map_err(|_err| FileError::InternalError("IO error".to_string()))?;

    while let Some(chunk) = resp
        .chunk()
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|_err| FileError::InternalError("IO error".to_string()))?
    {
        file.write_all(&chunk)
            .await
            .inspect_err(|err| log::error!("Write to temp file({temp_path:?}) error:{err}"))
            .map_err(|_err| FileError::InternalError("IO error".to_string()))?;
    }

    file.flush()
        .await
        .inspect_err(|err| log::error!("Save temp file({temp_path:?}) error: {err}"))
        .map_err(|_err| FileError::InternalError("IO error".to_string()))?;

    Ok(temp_path)
}

/// pull item from server by `NamedFile`
pub async fn pull_item_by_named_file(id: u64) -> Result<PathBuf, FileError> {
    let db = get_db_connection().await;
    let path_mapping = path_mappings::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            FileError::InternalError(format!("Query path mapping {id} failed: {err}"))
        })?;

    match path_mapping {
        Some(path_mapping) => {
            let url = Url::parse(path_mapping.path.as_str())
                .inspect_err(|err| log::error!("{err}"))
                .map_err(|_err| {
                    FileError::InternalError(format!("Parse url error, file_id = {id}"))
                })?;
            let scheme = url.scheme();
            match scheme {
                "file" => local_file_handler(&url)
                    .await
                    .inspect(|_| log::info!("pull file({id}) from {url}")),
                "http" => http_file_handler(&url, id)
                    .await
                    .inspect(|_| log::info!("pull file({id}) from {url}")),
                "https" => http_file_handler(&url, id)
                    .await
                    .inspect(|_| log::info!("pull file({id}) from {url}")),
                _ => Err(FileError::NotSupportedScheme(scheme.to_string())),
            }
        }
        None => Err(FileError::NotFound(id)),
    }
}

pub async fn push_item_link(link: Url) -> Result<u64, anyhow::Error> {
    let db = get_db_connection().await;
    let path_mapping = path_mappings::ActiveModel {
        path: Set(link.to_string()),
        mime_type: Set("external".to_string()),
        ..Default::default()
    };
    let path_mapping = path_mapping
        .insert(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| FileError::InternalError(format!("Insert path mapping failed: {err}")))?;
    Ok(path_mapping.pmid)
}

pub async fn push_item_octet_stream<S, E>(payload: S) -> Result<PushResponse, FileError>
where
    S: StreamExt<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Display,
{
    push_item_octet_stream_with_path(payload, &get_default_items_dir()?)
        .await
        .inspect_err(|err| log::error!("Upload file failed, error:{err}"))
}

pub async fn push_item_multipart_stream<S, E, T>(payload: S) -> Result<PushResponse, FileError>
where
    S: StreamExt<Item = Result<T, E>> + Unpin,
    T: StreamExt<Item = Result<Bytes, E>> + Unpin + ItemFile,
    E: std::fmt::Display,
{
    push_item_multipart_stream_with_path(payload, &get_default_items_dir()?)
        .await
        .inspect_err(|err| log::error!("Upload file failed, error:{err}"))
}

pub async fn push_item_multipart_stream_with_path<S, E, T, P>(
    mut payload: S,
    path: P,
) -> Result<PushResponse, FileError>
where
    S: StreamExt<Item = Result<T, E>> + Unpin,
    T: StreamExt<Item = Result<Bytes, E>> + Unpin + ItemFile,
    E: std::fmt::Display,
    P: AsRef<Path>,
{
    let mut path = path.as_ref().to_path_buf();
    path.push(uuid::Uuid::new_v4().to_string());
    let path: &Path = path.as_path();

    let mut guard = FileCleanupGuard::new();
    guard.add_dir(path);

    if !path.exists() {
        std::fs::create_dir_all(path)
            .inspect_err(|err| {
                log::error!("Failed to create upload directory {:?}: {err}", path);
            })
            .map_err(|err| match err.kind() {
                io::ErrorKind::StorageFull => FileError::InsufficientStorage,
                _ => FileError::InternalError(format!("Failed to create upload directory: {err}")),
            })?;
    }

    let base_dir = path
        .canonicalize()
        .inspect_err(|err| log::error!("Failed to canonicalize base upload directory: {err}"))
        .map_err(|err| {
            FileError::InternalError(format!("Failed to resolve base directory: {err}"))
        })?;

    let mut total_size = 0;

    while let Some(file) = payload.next().await {
        let mut payload = file
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| FileError::UploadFailed(err.to_string()))?;

        let name = payload
            .name()
            .ok_or(FileError::UploadFailed("Unamed file error".to_string()))?;

        let Ok(file_path) = check_new_path_safe(&base_dir, name) else {
            log::error!("Security block: Malicious relative path detected in tarball: {name}");
            return Err(FileError::PathTraversalAttack(name.to_string()));
        };

        if let Some(parent) = file_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)
                .inspect_err(|err| {
                    log::error!("Failed to create upload directory {:?}: {err}", parent);
                })
                .map_err(|err| match err.kind() {
                    io::ErrorKind::StorageFull => FileError::InsufficientStorage,
                    _ => FileError::InternalError(format!(
                        "Failed to create upload directory: {err}"
                    )),
                })?
        }

        let mut file = File::create(file_path)
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| match err.kind() {
                io::ErrorKind::StorageFull => FileError::InsufficientStorage,
                _ => FileError::InternalError(format!("Create temp file failed: {err}")),
            })?;

        let mut size = 0;

        while let Some(chunk) = payload.next().await {
            let chunk = chunk
                .inspect_err(|err| log::error!("{err}"))
                .map_err(|err| FileError::UploadFailed(err.to_string()))?;

            size += chunk.len() as u64;
            file.write_all(&chunk)
                .await
                .inspect_err(|err| log::error!("{err}"))
                .map_err(|err| match err.kind() {
                    io::ErrorKind::StorageFull => FileError::InsufficientStorage,
                    _ => FileError::InternalError(format!("Write to temp file failed: {err}")),
                })?;
        }
        total_size += size;
    }

    let pmid = store_path_mapping(&base_dir).await?;
    guard.disarm();

    Ok(PushResponse {
        pmid,
        uploaded_size: total_size,
    })
}

pub async fn push_item_octet_stream_with_path<S, E, P>(
    mut payload: S,
    path: P,
) -> Result<PushResponse, FileError>
where
    S: StreamExt<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Display,
    P: AsRef<Path>,
{
    let mut path = path.as_ref().to_path_buf();
    path.push(uuid::Uuid::new_v4().to_string());
    let path: &Path = path.as_path();

    let mut guard = FileCleanupGuard::new();
    guard.add_file(path);

    let mut file = File::create(path)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| match err.kind() {
            io::ErrorKind::StorageFull => FileError::InsufficientStorage,
            _ => FileError::InternalError(format!("Create temp file failed: {err}")),
        })?;
    let mut size = 0;

    while let Some(chunk) = payload.next().await {
        let chunk = chunk
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| FileError::UploadFailed(err.to_string()))?;

        size += chunk.len() as u64;
        file.write_all(&chunk)
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| match err.kind() {
                io::ErrorKind::StorageFull => FileError::InsufficientStorage,
                _ => FileError::InternalError(format!("Write to temp file failed: {err}")),
            })?;
    }

    let pmid = store_path_mapping(&path).await?;
    guard.disarm();

    Ok(PushResponse {
        pmid,
        uploaded_size: size,
    })
}
