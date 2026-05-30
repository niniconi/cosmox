use std::{
    fmt::Debug,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use cosmox_api::metadata::Metadata;
use cosmox_configuration::Configuration;
use sea_orm::EntityTrait;
use serde::Deserialize;

use crate::{entities::metadata_indexes, get_db_connection};

/// Errors related to metadata operations.
#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("Metadata not found with {0}")]
    NotFound(u64),

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

#[derive(Deserialize)]
pub struct MetadataQueryRequest {
    pub root_node: u64,
    pub depth: usize,
}

// #[instrument]
pub async fn load_metadata<P>(
    path: P,
    max_depth: usize,
) -> Result<Option<Arc<Mutex<Metadata<()>>>>, MetadataError>
where
    P: AsRef<Path> + Debug,
{
    let path = path.as_ref().to_path_buf();
    let mut root = None;
    let mut dirs = vec![(path, 1, None::<Arc<Mutex<Metadata<()>>>>)];

    while !dirs.is_empty()
        && let Some((mut path, depth, parent)) = dirs.pop()
    {
        let tmp_path = path.clone();
        path.push(".metadata");

        let file = File::open(&path);

        if let Err(err) = file {
            log::warn!("Open {path:?} error: {err}");
            continue;
        }

        let file = file.unwrap();
        let mut buffer_reader = BufReader::new(file);
        let metadata = Metadata::<()>::bindecode_from(&mut buffer_reader)
            .inspect_err(|err| log::error!("Decode metadata from {path:?} error:{err}"))
            .map_err(|err| {
                MetadataError::InternalError(format!("Decode metadata failed: {err}"))
            })?;

        if let Some(parent) = parent {
            parent.lock().unwrap().sub_metadatas.push(metadata.clone());
        } else {
            root = Some(metadata.clone());
        }

        if depth < max_depth {
            let sub_dirs = fs::read_dir(&tmp_path)
                .inspect_err(|err| log::error!("Failed to read directory {:?}: {err}", tmp_path))
                .map_err(|err| {
                    MetadataError::InternalError(format!("Failed to read subdirectory: {err}"))
                })?
                .filter_map(|entry| {
                    if let Ok(entry) = entry
                        && let Ok(entry_metadata) = entry.metadata()
                        && entry_metadata.is_dir()
                    {
                        Some((entry.path(), depth + 1, Some(metadata.clone())))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            dirs.extend(sub_dirs);
        }
    }

    Ok(root)
}

pub async fn query_metadata(
    query: Arc<MetadataQueryRequest>,
) -> Result<Arc<Mutex<Metadata<()>>>, MetadataError> {
    let db = get_db_connection().await;
    let metadata_index = metadata_indexes::Entity::find_by_id(query.root_node)
        .one(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            MetadataError::InternalError(format!(
                "Query metadata index {} failed: {err}",
                query.root_node
            ))
        })?;

    match metadata_index {
        Some(metadata_index) => {
            let mut metadata_path = PathBuf::from(
                &Configuration::get_global_configuration()
                    .cosmox
                    .scanner
                    .metadata_path,
            );
            metadata_path.push(metadata_index.path);
            load_metadata(&metadata_path, query.depth)
                .await
                .and_then(|metadata_tree| match metadata_tree {
                    Some(metadata_tree) => Ok(metadata_tree),
                    None => Err(MetadataError::NotFound(query.root_node)),
                })
        }
        None => Err(MetadataError::NotFound(query.root_node)),
    }
}
