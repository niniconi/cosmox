use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufWriter,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
};

use cosmox_api::metadata::Metadata;
use cosmox_configuration::{Configuration, ScannerConfiguration};
use futures_util::future::{join_all, try_join_all};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, QueryFilter};
use url::Url;

use serde::Serialize;

use crate::{
    entities::{metadata_indexes, prelude::*},
    get_db_connection,
    services::{
        file_service, resource_service,
        tag_service::{self, TagError},
    },
};

#[derive(Debug, Serialize)]
pub struct ScannerStatus {
    pub scanning: bool,
    pub current_lid: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ScannerInfo {
    pub available: bool,
    pub version: Option<String>,
}

pub async fn get_scanner_status() -> Result<ScannerStatus, ScannerError> {
    Ok(ScannerStatus {
        scanning: false,
        current_lid: None,
    })
}

pub async fn get_scanner_info() -> Result<ScannerInfo, ScannerError> {
    Ok(ScannerInfo {
        available: true,
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    })
}

#[derive(Debug, Clone)]
pub enum SelectedLibraries {
    ALL,
    SINGLE(u64),
}

type ScannerContext<'a> = (
    Arc<HashMap<u64, Vec<(String, Url)>>>,
    Arc<HashMap<u64, Vec<(String, String)>>>,
);

#[derive(Debug)]
pub struct ScannerContextInformation {
    pub lid: u64,
    pub library_paths: Vec<String>,
    pub library_type: String,
    pub global_config: Arc<ScannerConfiguration>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ScannerError {
    #[error("Target library '{0}' not found.")]
    NotFound(u64),

    #[error("Not authorized to manage scanners.")]
    Unauthorized,

    #[error("Scanner '{0}' is already running.")]
    AlreadyRunning(String),

    #[error("Scanner '{0}' failed to start: {1}")]
    StartFailed(String, String),

    #[error("Scanner task for '{0}' failed: {1}")]
    TaskFailed(String, String),

    #[error("Scanner '{0}' is not configured correctly: {1}")]
    InvalidConfiguration(String, String),

    #[error("Scan path '{0}' is invalid or inaccessible.")]
    InvalidScanPath(String),

    #[error("No available scanner instances to process the request.")]
    NoAvailableScanner,

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

/// prepare context information for scanner
pub async fn prepare_context_information(
    seleted: SelectedLibraries,
) -> Result<Vec<Arc<ScannerContextInformation>>, ScannerError> {
    let db = get_db_connection().await;
    match seleted {
        SelectedLibraries::ALL => {
            let libraries = Libraries::find()
                .all(db.as_ref())
                .await
                .inspect_err(|err| log::error!("{err}"))
                .map_err(|err| {
                    ScannerError::InternalError(format!("Query all libraries failed: {err}"))
                })?;

            // find all library paths for all libraries
            let find_all_paths_futures: Vec<_> = libraries
                .iter()
                .map(|library| async {
                    let library_paths = LibraryPaths::find()
                        .filter(LibraryPaths::COLUMN.lid.eq(library.lid))
                        .all(db.as_ref())
                        .await
                        .inspect_err(|err| log::error!("{err}"))
                        .map_err(|err| {
                            ScannerError::InternalError(format!(
                                "Query paths for library {} failed: {err}",
                                library.lid
                            ))
                        })?;
                    let library_paths = library_paths
                        .iter()
                        .map(|library_path| library_path.path.clone())
                        .collect();

                    let r#type = match Types::find_by_id(library.r#type).one(db.as_ref()).await {
                        Ok(r#type) => r#type.map(|x| x.label),
                        Err(err) => {
                            log::error!("{err}");
                            None
                        }
                    };

                    let lid = library.lid;
                    let library_type = r#type.unwrap_or("Default".to_string());
                    let global_config = Arc::new(
                        Configuration::get_global_configuration()
                            .cosmox
                            .scanner
                            .clone(),
                    );

                    Ok::<Arc<ScannerContextInformation>, ScannerError>(Arc::new(
                        ScannerContextInformation {
                            lid,
                            library_paths,
                            library_type,
                            global_config,
                        },
                    ))
                })
                .collect();

            Ok(try_join_all(find_all_paths_futures).await?)
        }
        SelectedLibraries::SINGLE(lid) => {
            let library = Libraries::find_by_id(lid)
                .one(db.as_ref())
                .await
                .inspect_err(|err| log::error!("{err}"))
                .map_err(|err| {
                    ScannerError::InternalError(format!("Query library {lid} failed: {err}"))
                })?;

            // find library paths from library {lid}
            let library_paths = LibraryPaths::find()
                .filter(LibraryPaths::COLUMN.lid.eq(lid))
                .all(db.as_ref())
                .await
                .inspect_err(|err| log::error!("{err}"))
                .map_err(|err| {
                    ScannerError::InternalError(format!(
                        "Query paths for library {lid} failed: {err}"
                    ))
                })?;
            let library_paths = library_paths
                .iter()
                .map(|library_path| library_path.path.clone())
                .collect();

            if let Some(library) = library {
                let r#type = match Types::find_by_id(library.r#type).one(db.as_ref()).await {
                    Ok(r#type) => r#type.map(|x| x.label),
                    Err(err) => {
                        log::error!("{err}");
                        None
                    }
                };

                let lid = library.lid;
                let library_type = r#type.unwrap_or("Default".to_string());
                let global_config = Arc::new(
                    Configuration::get_global_configuration()
                        .cosmox
                        .scanner
                        .clone(),
                );

                Ok(vec![Arc::new(ScannerContextInformation {
                    lid,
                    library_paths,
                    library_type,
                    global_config,
                })])
            } else {
                Err(ScannerError::NotFound(lid))
            }
        }
    }
}

/// store metadata tree to disk.
pub async fn store_metadata(
    lid: u64,
    metadata: Arc<Mutex<Metadata<()>>>,
    context: ScannerContext<'_>,
) -> Result<(), ScannerError> {
    let db = get_db_connection().await;
    let config = Configuration::get_global_configuration();
    let metadata_path = PathBuf::from(config.cosmox.scanner.metadata_path.as_str());
    if !metadata_path.exists()
        && let Err(err) = fs::create_dir_all(&metadata_path)
    {
        log::error!(
            "Failed to create metadata directory {:?}: {err}",
            metadata_path
        );
    }
    if let Err(err) = inner(lid, metadata, metadata_path, context, db, 0).await {
        log::error!("Failed to store metadata: {err}");
    }
    return Ok(());

    fn inner(
        lid: u64,
        metadata: Arc<Mutex<Metadata<()>>>,
        path: PathBuf,
        context: ScannerContext,
        db: Arc<DatabaseConnection>,
        level: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), ScannerError>> + Send>> {
        let (path_mapping, tags) = context;

        Box::pin(async move {
            let tmp_rid = metadata.lock().unwrap().rid;
            let mut inserted_tags: Vec<u64> = Vec::new();

            if let Some(path_mappings) = path_mapping.get(&tmp_rid) {
                let insert_path_mapping_futures = path_mappings
                    .iter()
                    .map(|(path_mapping, url)| {
                        store_path_mapping(metadata.clone(), path_mapping, url)
                    })
                    .collect::<Vec<_>>();
                join_all(insert_path_mapping_futures)
                    .await
                    .iter()
                    .for_each(|x| {
                        if let Err(err) = x {
                            log::error!("{err}");
                        }
                    });
            }

            if let Some(tags) = tags.get(&tmp_rid) {
                let insert_tag_futures = tags
                    .iter()
                    .map(|(group_label, label)| store_tag(group_label, label))
                    .collect::<Vec<_>>();

                inserted_tags = join_all(insert_tag_futures)
                    .await
                    .iter()
                    .inspect(|x| {
                        if let Err(err) = x {
                            log::error!("{err}");
                        }
                    })
                    .flatten()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>();
            }

            let metadata_snapshot = metadata.lock().unwrap().clone();
            let rid = resource_service::add_resource_by_metadata(
                lid,
                &metadata_snapshot,
                level,
                db.clone(),
            )
            .await
            .map_err(|err| {
                ScannerError::InternalError(format!(
                    "Add resource by metadata for library {lid} failed: {err}"
                ))
            })?;

            if !inserted_tags.is_empty() {
                let _ = resource_service::add_tags_for_resource(rid, inserted_tags).await;
            }

            let path = path.join(rid.to_string());
            let metadata_file_path = path.join(".metadata");
            if !path.exists()
                && let Err(err) = fs::create_dir_all(&path)
            {
                log::error!("Failed to create metadata subdirectory {:?}: {err}", path);
            }

            let file = File::create(metadata_file_path).unwrap();
            let mut writer = BufWriter::new(file);

            let inner_futures = {
                let mut metadata = metadata.lock().unwrap();
                metadata.rid = rid;
                if let Err(err) = metadata.encode_no_child_into_std_write(&mut writer) {
                    log::error!("Failed to encode metadata: {err}");
                }

                metadata
                    .sub_metadatas
                    .iter()
                    .map(|sub_metadata| {
                        inner(
                            lid,
                            sub_metadata.clone(),
                            path.clone(),
                            (path_mapping.clone(), tags.clone()),
                            db.clone(),
                            level + 1,
                        )
                    })
                    .collect::<Vec<_>>()
            };

            let metadata_store_path: &String = &Configuration::get_global_configuration()
                .cosmox
                .scanner
                .metadata_path;

            match path.strip_prefix(metadata_store_path) {
                Ok(path) => {
                    let metadata_relative_path = path.to_string_lossy().into_owned();
                    let metadata_index = metadata_indexes::ActiveModel {
                        path: Set(metadata_relative_path),
                        mid: Set(rid),
                    };
                    if let Err(err) = metadata_index.insert(db.as_ref()).await {
                        log::error!("Failed to insert metadata_index: {err}");
                    }
                }
                Err(err) => {
                    log::warn!("Can't insert metadata_index to database.");
                    log::error!("{err}");
                }
            }

            join_all(inner_futures).await;
            Ok(())
        })
    }
}
async fn store_tag(group_label: &str, label: &str) -> Result<Vec<u64>, ScannerError> {
    let mut inserted_tags: Vec<u64> = Vec::new();
    log::debug!("insert tag {group_label}:{label}");

    // insert or get id of tag_group
    let tgid = match tag_service::add_tag_group(group_label.to_string()).await {
        Ok(tgid) => tgid,
        Err(err) => match err {
            TagError::AlreadyExists(_) => {
                if let Ok(Some(tag_group)) =
                    tag_service::get_tag_group_by_label(group_label.to_string()).await
                {
                    tag_group.tgid
                } else {
                    return Err(ScannerError::InternalError(
                        "Failed to get tag group after insert conflict".to_string(),
                    ));
                }
            }
            _ => {
                return Err(ScannerError::InternalError(format!(
                    "Insert tag group failed: {err}"
                )));
            }
        },
    };

    // insert tag
    match tag_service::add_tag(label.to_string(), tgid).await {
        Ok(tag_id) => {
            inserted_tags.push(tag_id);
        }
        Err(err) => {
            if matches!(err, TagError::AlreadyExists(_)) {
                if let Ok(Some(tag)) = tag_service::get_tag_by_label(label.to_string()).await {
                    inserted_tags.push(tag.tid);
                } else {
                    return Err(ScannerError::InternalError(
                        "Failed to get tag after insert conflict".to_string(),
                    ));
                }
            } else {
                return Err(ScannerError::InternalError(format!(
                    "Insert tag {label} in group {tgid} failed: {err}"
                )));
            }
        }
    };
    Ok(inserted_tags)
}

async fn store_path_mapping(
    metadata: Arc<Mutex<Metadata<()>>>,
    field: &str,
    url: &Url,
) -> Result<(), ScannerError> {
    log::debug!("Insert url:{url:?} into database");

    let pmid = file_service::push_item_link(url.clone())
        .await
        .map_err(|err| ScannerError::InternalError(format!("Push item link failed: {err}")))?;

    match field {
        "cover_file_map_id" => metadata.lock().unwrap().cover_file_map_id = Some(pmid),
        "data_file_map_id" => metadata.lock().unwrap().data_file_map_id = Some(pmid),
        s if s.starts_with(':') => {
            todo!();
        }
        _ => {}
    }
    Ok(())
}
