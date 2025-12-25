use std::{path::Path, sync::Arc};

use actix_files::NamedFile;
use actix_web::web::Payload;
use futures::StreamExt;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};
use tokio::{
  fs::File,
  io::{self, AsyncWriteExt},
};
use url::Url;

use crate::{core::io::file_controller::FileError, entities::path_mappings};

async fn local_file_handler(url: &Url, id: u64) -> Result<NamedFile, FileError> {
  NamedFile::open_async(url.path())
    .await
    .inspect_err(|err| log::error!("Open named file error: {err}"))
    .map_err(|err| FileError::InternalError("IO error".to_string()))
}

async fn http_file_handler(url: &Url, id: u64) -> Result<NamedFile, FileError> {
  let mut resp = reqwest::get(url.as_str())
    .await
    .inspect_err(|err| log::error!("Download file({id}) {url} failed: {err}"))
    .map_err(|err| FileError::DownloadFailed(url.to_string()))?;

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
    .map_err(|err| FileError::InternalError("IO error".to_string()))?;

  while let Some(chunk) = resp
    .chunk()
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|err| FileError::InternalError("IO error".to_string()))?
  {
    file
      .write_all(&chunk)
      .await
      .inspect_err(|err| log::error!("Write to temp file({temp_path:?}) error:{err}"))
      .map_err(|err| FileError::InternalError("IO error".to_string()))?;
  }

  file
    .flush()
    .await
    .inspect_err(|err| log::error!("Save temp file({temp_path:?}) error: {err}"))
    .map_err(|err| FileError::InternalError("IO error".to_string()))?;

  NamedFile::open_async(temp_path)
    .await
    .inspect_err(|err| log::error!("Open named file error: {err}"))
    .map_err(|err| FileError::InternalError("IO error".to_string()))
}

/// pull item from server by `NamedFile`
pub async fn pull_item_by_named_file(
  id: u64,
  db: Arc<DatabaseConnection>,
) -> Result<NamedFile, FileError> {
  let path_mapping = path_mappings::Entity::find_by_id(id)
    .one(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|err| FileError::InternalError("Database error".to_string()))?;

  match path_mapping {
    Some(path_mapping) => {
      let url = Url::parse(path_mapping.path.as_str())
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| FileError::InternalError(format!("Parse url error, file_id = {id}")))?;
      let scheme = url.scheme();
      match scheme {
        "file" => local_file_handler(&url, id)
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

pub async fn push_item_link(link: Url, db: Arc<DatabaseConnection>) -> Result<u64, anyhow::Error> {
  let path_mapping = path_mappings::ActiveModel {
    path: Set(link.to_string()),
    mime_type: Set("external".to_string()),
    ..Default::default()
  };
  let path_mapping = path_mapping
    .insert(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|err| FileError::InternalError("Unknown error".to_string()))?;
  Ok(path_mapping.pmid)
}

pub async fn push_item_octet_stream(
  payload: Payload,
  db: Arc<DatabaseConnection>,
) -> Result<usize, FileError> {
  // TODO set a default directory.
  push_item_octet_stream_with_path(payload, "", db)
    .await
    .inspect_err(|err| log::error!("Upload file failed, error:{err}"))
}

pub async fn push_item_octet_stream_with_path<P>(
  mut payload: Payload,
  path: P,
  db: Arc<DatabaseConnection>,
) -> Result<usize, FileError>
where
  P: AsRef<Path>,
{
  let path = path.as_ref();
  let mut file = File::create(path)
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|err| match err.kind() {
      io::ErrorKind::StorageFull => FileError::InsufficientStorage,
      _ => FileError::InternalError("Unknown error".to_string()),
    })?;
  let mut size = 0;

  while let Some(chunk) = payload.next().await {
    let chunk = chunk
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| FileError::InternalError("Unknown error".to_string()))?;
    size += chunk.len();
    file
      .write_all(&chunk)
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|err| match err.kind() {
        io::ErrorKind::StorageFull => FileError::InsufficientStorage,
        _ => FileError::InternalError("Unknown error".to_string()),
      })?;
  }

  let url_string = format!(
    "file://{}",
    path
      .canonicalize()
      .map_err(|_err| FileError::InternalError("Unknown Error".to_string()))?
      .to_str()
      .ok_or(FileError::InternalError("Unknown Error".to_string()))?
      .to_string()
  );

  let url = Url::parse(url_string.as_str())
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| FileError::InternalError(format!("Parse url error path: {path:?}")))?;

  push_item_link(url, db)
    .await
    .map_err(|err| FileError::InternalError(err.to_string()))?;

  Ok(size)
}
