use std::{
    fmt::Debug,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use cosmox_api::metadata::{Metadata, MetadataNode, MetadataType};
use cosmox_configuration::Configuration;
use sea_orm::EntityTrait;

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

/// The root of a metadata query: a concrete node id, or the tree root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataQueryKey {
    /// Query a concrete node by id.
    Id(u64),
    /// Query the root of the metadata tree.
    Root,
}

pub struct MetadataQueryRequest {
    pub root: MetadataQueryKey,
    pub depth: usize,
}

// #[instrument]
pub async fn load_metadata<P>(
    path: P,
    max_depth: usize,
) -> Result<Option<MetadataNode>, MetadataError>
where
    P: AsRef<Path> + Debug,
{
    let path = path.as_ref().to_path_buf();
    let mut root = None;
    let mut dirs = vec![(path, 1, None::<MetadataNode>)];

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

async fn load_root_children(
    metadata_path: &Path,
    max_depth: usize,
    root: &MetadataNode,
) -> Result<(), MetadataError> {
    let entries = fs::read_dir(metadata_path)
        .inspect_err(|err| log::error!("Failed to read directory {:?}: {err}", metadata_path))
        .map_err(|err| {
            MetadataError::InternalError(format!("Failed to read metadata root: {err}"))
        })?
        .filter_map(|entry| {
            if let Ok(entry) = entry
                && let Ok(entry_metadata) = entry.metadata()
                && entry_metadata.is_dir()
            {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for dir in entries {
        if let Some(child) = load_metadata(&dir, max_depth).await? {
            root.lock().unwrap().sub_metadatas.push(child);
        }
    }
    Ok(())
}

pub async fn query_metadata(
    query: Arc<MetadataQueryRequest>,
) -> Result<MetadataNode, MetadataError> {
    let db = get_db_connection().await;
    let metadata_path = PathBuf::from(
        &Configuration::get_global_configuration()
            .cosmox
            .scanner
            .metadata_path,
    );

    match query.root {
        // Root is a synthetic container: each direct subdirectory of
        // metadata_path is a top-level metadata node carrying its own
        // `.metadata`, so the index table is not consulted.
        MetadataQueryKey::Root => {
            let root = Arc::new(Mutex::new(Metadata::<()> {
                name: "root".into(),
                metadata_type: MetadataType::Directory,
                ..Default::default()
            }));
            load_root_children(&metadata_path, query.depth, &root).await?;
            Ok(root)
        }
        MetadataQueryKey::Id(root_node) => {
            let metadata_index = metadata_indexes::Entity::find_by_id(root_node)
                .one(db.as_ref())
                .await
                .inspect_err(|err| log::error!("{err}"))
                .map_err(|err| {
                    MetadataError::InternalError(format!(
                        "Query metadata index {root_node} failed: {err}"
                    ))
                })?;

            match metadata_index {
                Some(metadata_index) => {
                    let mut path = metadata_path;
                    path.push(metadata_index.path);
                    load_metadata(&path, query.depth)
                        .await
                        .and_then(|metadata_tree| match metadata_tree {
                            Some(metadata_tree) => Ok(metadata_tree),
                            None => Err(MetadataError::NotFound(root_node)),
                        })
                }
                None => Err(MetadataError::NotFound(root_node)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static NEXT_TMP: AtomicU64 = AtomicU64::new(0);

    fn make_temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cosmox_metadata_test_{}_{}",
            std::process::id(),
            NEXT_TMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_metadata_file(dir: &Path, name: &str) {
        let metadata = Metadata::<()> {
            rid: 1,
            name: name.into(),
            ..Default::default()
        };
        let path = dir.join(".metadata");
        let mut writer = std::io::BufWriter::new(File::create(path).unwrap());
        metadata
            .encode_no_child_into_std_write(&mut writer)
            .unwrap();
        writer.flush().unwrap();
    }

    #[tokio::test]
    async fn load_root_children_collects_top_level_dirs() {
        let root_dir = make_temp_dir().join("metadata_root");
        fs::create_dir_all(&root_dir).unwrap();
        let child_a = root_dir.join("100");
        let child_b = root_dir.join("200");
        fs::create_dir_all(&child_a).unwrap();
        fs::create_dir_all(&child_b).unwrap();
        write_metadata_file(&child_a, "a");
        write_metadata_file(&child_b, "b");

        let root = Arc::new(Mutex::new(Metadata::<()>::default()));
        load_root_children(&root_dir, 1, &root).await.unwrap();

        let children = root.lock().unwrap().sub_metadatas.clone();
        assert_eq!(children.len(), 2);
        let mut names = children
            .iter()
            .map(|c| c.lock().unwrap().name.clone())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn load_root_children_empty_root() {
        let dir = make_temp_dir();

        let root = Arc::new(Mutex::new(Metadata::<()>::default()));
        load_root_children(&dir, 1, &root).await.unwrap();

        assert!(root.lock().unwrap().sub_metadatas.is_empty());
    }
}
