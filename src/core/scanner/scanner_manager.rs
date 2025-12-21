use std::{
  collections::HashMap,
  fs::{self, File},
  io::BufWriter,
  path::{Path, PathBuf},
  pin::Pin,
  sync::{Arc, Mutex, atomic::AtomicU64},
};

use futures::future::{join_all, try_join_all};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use url::Url;
use wasmtime::component::Resource;

use crate::{
  configuration::{Configuration, ScannerConfiguration},
  controller::tag_controller::TagError,
  core::{
    io::file_service,
    plugin::{
      context::event::{
        MetadataContext, PathMappingContext, PathMappingContextTemp, TagContext, TagContextTemp,
      },
      plugin_loader::bindings::cosmox::plugin::context::{self as bindings_context},
      plugin_manager::PluginManager,
    },
    scanner::controller::scanner_controller::ScannerError,
  },
  entities::{library_paths, librarys},
  services::{resource_service, tag_service},
};
use cosmox_api::metadata::{Metadata, MetadataType};

pub enum SelectedLibraries {
  ALL,
  SINGLE(u64),
}

#[derive(Debug)]
pub struct ScannerContextInformation {
  pub lid: u64,
  pub library_paths: Vec<String>,
  pub library_type: String,
  pub global_config: Arc<ScannerConfiguration>,
}

/// prepare context information for scanner
pub async fn prepare_context_information(
  seleted: SelectedLibraries,
  db: Arc<DatabaseConnection>,
) -> Result<Vec<Arc<ScannerContextInformation>>, ScannerError> {
  match seleted {
    SelectedLibraries::ALL => {
      let libraries = librarys::Entity::find().all(db.as_ref()).await.unwrap();

      // find all library paths for all libraries
      let find_all_paths_futures: Vec<_> = libraries
        .iter()
        .map(|library| async {
          let library_paths = library_paths::Entity::find()
            .filter(library_paths::Column::Lid.eq(library.lid))
            .all(db.as_ref())
            .await
            .unwrap();
          let library_paths = library_paths
            .iter()
            .map(|library_path| library_path.path.clone())
            .collect();

          Ok::<Arc<ScannerContextInformation>, ScannerError>(Arc::new(ScannerContextInformation {
            lid: library.lid,
            library_paths: library_paths,
            library_type: "Default".to_string(),
            global_config: Arc::new(
              Configuration::get_global_configuration()
                .cosmox
                .scanner
                .clone(),
            ),
          }))
        })
        .collect();

      Ok(try_join_all(find_all_paths_futures).await?)
    }
    SelectedLibraries::SINGLE(lid) => {
      let library = librarys::Entity::find_by_id(lid)
        .one(db.as_ref())
        .await
        .unwrap();

      // find library paths from library {lid}
      let library_paths = library_paths::Entity::find()
        .filter(library_paths::Column::Lid.eq(lid))
        .all(db.as_ref())
        .await
        .unwrap();
      let library_paths = library_paths
        .iter()
        .map(|library_path| library_path.path.clone())
        .collect();

      if let Some(library) = library {
        Ok(vec![Arc::new(ScannerContextInformation {
          lid: library.lid,
          library_paths: library_paths,
          library_type: "Default".to_string(),
          global_config: Arc::new(
            Configuration::get_global_configuration()
              .cosmox
              .scanner
              .clone(),
          ),
        })])
      } else {
        Err(ScannerError::NotFound(lid))
      }
    }
  }
}

/// start scanner
pub async fn start_scanner(
  scanner_context: Arc<ScannerContextInformation>,
  db: Arc<DatabaseConnection>,
) -> Result<(), ScannerError> {
  log::debug!("start scanner ");
  let path_mapping = PathMappingContextTemp::default();
  let tag_temp = TagContextTemp::default();
  let metadata = match generate_metadata_tree(&scanner_context.library_paths) {
    Ok((metadata, count)) => {
      log::debug!("metadata tree been ready");
      let count = Arc::new(AtomicU64::new(count));

      let event = Arc::new(cosmox_api::Event::OnMetadataRawTreeReady(
        cosmox_api::EventPayload::Data(()),
      ));

      PluginManager::notify_all(event, |store_mut_ref| {
        let resource_metadata_context: Resource<MetadataContext> =
          bindings_context::HostMetadataHandle::new(store_mut_ref.data_mut()).unwrap();

        let mut_ref_metadata_context = store_mut_ref
          .data_mut()
          .resource_table
          .get_mut(&resource_metadata_context)
          .unwrap();
        mut_ref_metadata_context.inner = Some(metadata.clone());
        mut_ref_metadata_context.count = count.clone();

        bindings_context::EventContext::MetadataReadyContext(resource_metadata_context)
      })
      .await;

      let event = Arc::new(cosmox_api::Event::OnMetadataLocalTreeReady(
        cosmox_api::EventPayload::Data(()),
      ));

      PluginManager::notify_all(event, |store_mut_ref| {
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
      .await;

      metadata
    }
    Err(err) => {
      todo!()
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
  store_metadata(scanner_context.lid, metadata, (path_mapping, tag_temp), db).await
}

type ScannerContext<'a> = (
  Arc<HashMap<u64, Vec<(String, Url)>>>,
  Arc<HashMap<u64, Vec<(String, String)>>>,
);

/// store metadata tree to disk.
pub async fn store_metadata(
  lid: u64,
  metadata: Arc<Mutex<Metadata<()>>>,
  context: ScannerContext<'_>,
  db: Arc<DatabaseConnection>,
) -> Result<(), ScannerError> {
  let config = Configuration::get_global_configuration();
  let metadata_path = PathBuf::from(config.cosmox.scanner.metadata_path.as_str());
  if !metadata_path.exists() {
    fs::create_dir_all(&metadata_path);
  }
  inner(lid, metadata, metadata_path, context, db).await;
  return Ok(());

  fn inner(
    lid: u64,
    metadata: Arc<Mutex<Metadata<()>>>,
    path: PathBuf,
    context: ScannerContext,
    db: Arc<DatabaseConnection>,
  ) -> Pin<Box<dyn Future<Output = Result<(), ScannerError>>>> {
    let rid = metadata.lock().unwrap().rid;
    let path = path.join(rid.to_string());
    let (path_mapping, tags) = context;
    let metadata_file_path = path.join(".metadata");
    if !path.exists() {
      fs::create_dir_all(&path);
    }

    Box::pin(async move {
      let rid = metadata.lock().unwrap().rid;
      let mut inserted_tags: Vec<u64> = Vec::new();

      if let Some(path_mappings) = path_mapping.get(&rid) {
        let insert_path_mapping_futures = path_mappings
          .iter()
          .map(|(path_mapping, url)| {
            store_path_mapping(metadata.clone(), path_mapping, url, db.clone())
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

      if let Some(tags) = tags.get(&rid) {
        let insesrt_tag_futures = tags
          .iter()
          .map(|(group_label, label)| store_tag(group_label, label, db.clone()))
          .collect::<Vec<_>>();

        inserted_tags = join_all(insesrt_tag_futures)
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

      let rid = resource_service::add_resource_by_metadata(
        lid,
        &*metadata.lock().unwrap(),
        "".to_string(),
        db.clone(),
      )
      .await
      .map_err(|_err| ScannerError::InternalError("Database error".to_string()))?;

      if (!inserted_tags.is_empty()) {
        let _ = resource_service::add_tags_for_resource(rid, inserted_tags, db.clone()).await;
      }

      let file = File::create(metadata_file_path).unwrap();
      let mut writer = BufWriter::new(file);

      let mut metadata = metadata.lock().unwrap();
      metadata.rid = rid;
      metadata.encode_no_child_into_std_write(&mut writer);

      let inner_futures = metadata
        .sub_metadatas
        .iter()
        .map(|sub_metadata| {
          inner(
            lid,
            sub_metadata.clone(),
            path.clone(),
            (path_mapping.clone(), tags.clone()),
            db.clone(),
          )
        })
        .collect::<Vec<_>>();

      join_all(inner_futures).await;

      drop(metadata);
      Ok(())
    })
  }
}

async fn store_tag(
  group_label: &str,
  label: &str,
  db: Arc<DatabaseConnection>,
) -> Result<Vec<u64>, ScannerError> {
  let mut inserted_tags: Vec<u64> = Vec::new();
  log::debug!("insert tag {group_label}:{label}");

  // insert or get id of tag_group
  let tgid = match tag_service::add_tag_group(group_label.to_string(), db.clone()).await {
    Ok(tgid) => tgid,
    Err(err) => match err {
      TagError::AlreadyExists(_) => {
        if let Ok(Some(tag_group)) =
          tag_service::get_tag_group_by_label(group_label.to_string(), db.clone()).await
        {
          tag_group.tgid
        } else {
          return Err(ScannerError::InternalError("Unknown error".to_string()));
        }
      }
      _ => return Err(ScannerError::InternalError("Unknown error".to_string())),
    },
  };

  // insert tag
  match tag_service::add_tag(label.to_string(), tgid, db.clone()).await {
    Ok(tag_id) => {
      inserted_tags.push(tag_id);
    }
    Err(err) => {
      if matches!(err, TagError::AlreadyExists(_)) {
        if let Ok(Some(tag)) = tag_service::get_tag_by_label(label.to_string(), db.clone()).await {
          inserted_tags.push(tag.tid);
        } else {
          return Err(ScannerError::InternalError("Unknown error".to_string()));
        }
      } else {
        return Err(ScannerError::InternalError("Unknown error".to_string()));
      }
    }
  };
  Ok(inserted_tags)
}

async fn store_path_mapping(
  metadata: Arc<Mutex<Metadata<()>>>,
  field: &str,
  url: &Url,
  db: Arc<DatabaseConnection>,
) -> Result<(), ScannerError> {
  log::debug!("insert {:?}", "???");

  let pmid = file_service::push_item_link(url.clone(), db.clone())
    .await
    .map_err(|_err| ScannerError::InternalError("Unknown error".to_string()))?;

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

pub fn generate_metadata_tree<P>(
  paths: &[P],
) -> Result<(Arc<Mutex<Metadata<()>>>, u64), std::io::Error>
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

        let child_metadata = Arc::new(Mutex::new(Metadata::<()> {
          rid: count,
          name: entry.file_name().to_str().unwrap_or("Error").to_string(),
          url: "file:".to_string() + entry.path().to_str().unwrap_or("Error"),
          metadata_type: metadata_type,
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
    let metadata = generate_metadata_tree(&vec!["test/data/library1"]).unwrap();
    println!("{metadata:#?}");
  }

  #[test]
  fn test_store_metadata() {}
}
