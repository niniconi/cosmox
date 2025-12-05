use std::{
  fs::{self},
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
  thread::sleep,
  time::Duration,
};

use futures::future::try_join_all;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
  configuration::{Configuration, ScannerConfiguration},
  core::{
    plugin::plugin_manager::PluginManager, scanner::controller::scanner_controller::ScannerError,
  },
  entities::{library_paths, librarys},
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
) -> Result<(), ScannerError> {
  log::debug!("start scanner ");
  match generate_metadata_tree(&scanner_context.library_paths) {
    Ok(metadata) => {
      log::debug!("metadata tree been ready");
      let event = Arc::new(cosmox_api::Event::OnMetadataTreeReady(
        cosmox_api::EventPayload::Data(metadata),
      ));
      PluginManager::notify_all(event).await;
      Ok(())
    }
    Err(err) => {
      todo!()
    }
  }
}

pub fn generate_metadata_tree<P>(paths: &[P]) -> Result<Arc<Metadata<()>>, std::io::Error>
where
  P: AsRef<Path>,
{
  let metadata_tree = Arc::new(Metadata::default());

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

        let child_metadata = Arc::new(Metadata::<()> {
          name: Mutex::new(entry.file_name().to_str().unwrap_or("Error").to_string()),
          url: Mutex::new("file:".to_string() + entry.path().to_str().unwrap_or("Error")),
          metadata_type: metadata_type,
          ..Default::default()
        });

        parent_metadata
          .sub_metadatas
          .lock()
          .unwrap()
          .push(child_metadata.clone());

        if entry_metadata.is_dir() {
          dirs.push((entry.path(), child_metadata.clone()));
          // } else if entry_metadata.is_file() {
        }
      }
    }
  }
  Ok(metadata_tree.clone())
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn generate_metadata_tree_from_cosmox_api_directory() {
    let metadata = generate_metadata_tree(&vec!["cosmox-api"]).unwrap();
    println!("{metadata:#?}");
  }
}
