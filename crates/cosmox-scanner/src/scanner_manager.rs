use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, atomic::AtomicU64},
};

use cosmox_api::{
    event::{
        Event, EventPayload,
        payloads::{OnMetadataLocalTreeReadyEventContext, OnMetadataRawTreeReadyEventContext},
    },
    metadata::{Metadata, MetadataType},
};
use cosmox_backend_data::services::scanner_service::{self, store_metadata};
use cosmox_plugin_manager::{Resource, plugin_manager::bindings_context};
use cosmox_plugin_manager::{
    context::event::{
        MetadataContext, PathMappingContext, PathMappingContextTemp, TagContext, TagContextTemp,
    },
    plugin_manager::PluginManager,
};

pub use cosmox_backend_data::services::scanner_service::{
    ScannerContextInformation, ScannerError, SelectedLibraries,
};

#[derive(Debug, Clone)]
pub enum ScannerStatus {
    Running { task_count: usize, completed: usize },
    Stop,
    Err(ScannerError),
}

pub struct ScannerRuntimeContext {
    pub context: ScannerContextInformation,
    pub scanner_status: ScannerStatus,
}

// static SCANNER_STATE: LazyLock<RwLock<Vec<ScannerRuntimeContext>>> = LazyLock::new(|| {
//     RwLock::new(Vec::with_capacity(32))
// });
type GeneratedMetadataTree = (Arc<Mutex<Metadata<()>>>, u64);

pub async fn start(selected: SelectedLibraries) -> Result<(), ScannerError> {
    let contexts = scanner_service::prepare_context_information(selected.clone()).await?;

    if contexts.is_empty()
        && let SelectedLibraries::SINGLE(lid) = selected
    {
        return Err(ScannerError::NotFound(lid));
    }

    for context in contexts {
        log::info!("Created scan task by context {context:#?}");
        if let Err(err) = start_scanner(context.clone()).await {
            log::error!("{err}");
        }
    }

    Ok(())
}

/// start scanner
pub async fn start_scanner(
    scanner_context: Arc<ScannerContextInformation>,
) -> Result<(), ScannerError> {
    log::debug!("Start scanner ");
    let path_mapping = PathMappingContextTemp::default();
    let tag_temp = TagContextTemp::default();
    let metadata = match generate_metadata_tree(&scanner_context.library_paths) {
        Ok((metadata, count)) => {
            log::debug!("Metadata tree been ready");
            let count = Arc::new(AtomicU64::new(count));

            let event = Arc::new(Event::OnMetadataRawTreeReady(EventPayload::Data(
                OnMetadataRawTreeReadyEventContext {
                    lid: scanner_context.lid,
                    r#type: scanner_context.library_type.clone(),
                },
            )));

            let _ = PluginManager::notify_all(event, |store_mut_ref| {
                let resource_metadata_context: Resource<MetadataContext> =
                    bindings_context::HostMetadataHandle::new(store_mut_ref.data_mut()).unwrap();
                let resource_path_mapping_context: Resource<PathMappingContext> =
                    bindings_context::HostPathMappingHandle::new(store_mut_ref.data_mut()).unwrap();

                let mut_ref_metadata_context = store_mut_ref
                    .data_mut()
                    .resource_table
                    .get_mut(&resource_metadata_context)
                    .unwrap();
                mut_ref_metadata_context.inner = Some(metadata.clone());
                mut_ref_metadata_context.count = count.clone();

                let mut_ref_path_mapping_context = store_mut_ref
                    .data_mut()
                    .resource_table
                    .get_mut(&resource_path_mapping_context)
                    .unwrap();
                mut_ref_path_mapping_context.path_mapping_temp = path_mapping.clone();

                bindings_context::EventContext::MetadataReadyContext((
                    resource_metadata_context,
                    resource_path_mapping_context,
                ))
            })
            .await
            .inspect_err(|e| log::error!("Failed to notify OnMetadataRawTreeReady event: {e}"));

            let event = Arc::new(Event::OnMetadataLocalTreeReady(EventPayload::Data(
                OnMetadataLocalTreeReadyEventContext {
                    lid: scanner_context.lid,
                    r#type: scanner_context.library_type.clone(),
                    from_plugins: vec![],
                },
            )));

            let _ = PluginManager::notify_all(event, |store_mut_ref| {
                let resource_metadata_context: Resource<MetadataContext> =
                    bindings_context::HostMetadataHandle::new(store_mut_ref.data_mut()).unwrap();
                let resource_path_mapping_context: Resource<PathMappingContext> =
                    bindings_context::HostPathMappingHandle::new(store_mut_ref.data_mut()).unwrap();
                let resource_tag_context: Resource<TagContext> =
                    bindings_context::HostTagHandle::new(store_mut_ref.data_mut()).unwrap();

                let mut_ref_metadata_context = store_mut_ref
                    .data_mut()
                    .resource_table
                    .get_mut(&resource_metadata_context)
                    .unwrap();
                mut_ref_metadata_context.inner = Some(metadata.clone());
                mut_ref_metadata_context.count = count.clone();

                let mut_ref_path_mapping_context = store_mut_ref
                    .data_mut()
                    .resource_table
                    .get_mut(&resource_path_mapping_context)
                    .unwrap();
                mut_ref_path_mapping_context.path_mapping_temp = path_mapping.clone();

                let mut_ref_tag_context = store_mut_ref
                    .data_mut()
                    .resource_table
                    .get_mut(&resource_tag_context)
                    .unwrap();
                mut_ref_tag_context.tag_temp = tag_temp.clone();

                bindings_context::EventContext::MetadataLocalReadyContext((
                    resource_metadata_context,
                    resource_path_mapping_context,
                    resource_tag_context,
                ))
            })
            .await
            .inspect_err(|e| log::error!("Failed to notify OnMetadataLocalTreeReady event: {e}"));

            metadata
        }
        Err(err) => {
            todo!("scan metadata fallback error: {err}")
        }
    };

    let path_mapping = match Arc::try_unwrap(path_mapping) {
        Ok(mutex) => Arc::new(mutex.into_inner().unwrap()),
        Err(origin) => {
            let guard = origin.lock().unwrap();
            let path_mapping = Arc::new(guard.clone());
            log::warn!(
                "`path_mapping` has been cloned. The reference count is {}",
                Arc::strong_count(&origin)
            );
            drop(guard);
            path_mapping
        }
    };
    let tag_temp = match Arc::try_unwrap(tag_temp) {
        Ok(mutex) => Arc::new(mutex.into_inner().unwrap()),

        Err(origin) => {
            let guard = origin.lock().unwrap();
            let tag_temp = Arc::new(guard.clone());
            log::warn!(
                "`tag_temp` has been cloned. The reference count is {}",
                Arc::strong_count(&origin)
            );
            drop(guard);
            tag_temp
        }
    };
    store_metadata(scanner_context.lid, metadata, (path_mapping, tag_temp)).await
}

pub fn generate_metadata_tree<P>(paths: &[P]) -> Result<GeneratedMetadataTree, std::io::Error>
where
    P: AsRef<Path>,
{
    let mut count: u64 = 1;
    let metadata_tree = Arc::new(Mutex::new(Metadata::default()));

    let mut dirs = paths
        .iter()
        .map(|x| {
            let path = x.as_ref();
            (PathBuf::from(path), metadata_tree.clone())
        })
        .collect::<Vec<_>>();

    while !dirs.is_empty()
        && let Some((path, parent_metadata)) = dirs.pop()
    {
        let entries = fs::read_dir(path)?;

        for entry in entries {
            if let Ok(entry) = entry
                && let Ok(entry_metadata) = entry.metadata()
            {
                let metadata_type = if entry_metadata.is_file() {
                    MetadataType::File
                } else if entry_metadata.is_dir() {
                    MetadataType::Directory
                } else {
                    MetadataType::default()
                };
                let rid = count; // use the count as the ID
                let name = entry.file_name().to_str().unwrap_or("Error").to_string();
                let url = "file:".to_string() + entry.path().to_str().unwrap_or("Error");

                let child_metadata = Arc::new(Mutex::new(Metadata::<()> {
                    rid,
                    name,
                    url,
                    metadata_type,
                    ..Default::default()
                }));
                count += 1;

                parent_metadata
                    .lock()
                    .unwrap()
                    .sub_metadatas
                    .push(child_metadata.clone());

                if entry_metadata.is_dir() {
                    dirs.push((entry.path(), child_metadata.clone()));
                    // } else if entry_metadata.is_file() {
                }
            }
        }
    }
    Ok((metadata_tree.clone(), count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_metadata_tree_from_cosmox_api_directory() {
        let metadata = generate_metadata_tree(&["test/data/library1"]).unwrap();
        println!("{metadata:#?}");
    }

    #[test]
    fn test_store_metadata() {}
}
