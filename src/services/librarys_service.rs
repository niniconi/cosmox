use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use futures::future::try_join_all;
use sea_orm::{
  ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, TransactionTrait,
};

use crate::{
  controller::library_controller::LibraryAddRequest,
  entities::{library_paths, librarys, librarys_related_tags},
};

/// create library with tags and paths
/// # Arguments
/// - `Arc<LibraryAddRequest>`
/// - `Arc<DatabaseConnection>`
pub async fn create_library_with_tags_and_paths(
  payload: Arc<LibraryAddRequest>,
  db: Arc<DatabaseConnection>,
) -> Result<(
  librarys::Model,
  Vec<librarys_related_tags::Model>,
  Vec<library_paths::Model>,
)> {
  // check

  // insert into database
  let result = db
    .clone()
    .transaction::<_, (librarys::Model, Vec<_>, Vec<_>), anyhow::Error>(|_txn| {
      Box::pin(async move {
        let current_datetime = Utc::now().naive_utc();
        let library = librarys::ActiveModel {
          name: Set(Some(payload.name.clone())),
          description: Set(payload.description.clone()),
          create_datetime: Set(current_datetime),
          last_update_datetime: Set(current_datetime),
          ..Default::default()
        };
        let library = library.insert(db.as_ref()).await?;
        let add_tag_futures: Vec<_> = payload
          .tags
          .iter()
          .map(|x| async {
            let library_tag_relation = librarys_related_tags::ActiveModel {
              lid: Set(library.lid),
              tid: Set(*x),
              ..Default::default()
            };
            library_tag_relation
              .insert(db.as_ref())
              .await
              .map_err(|_err| anyhow::anyhow!("error"))
          })
          .collect();

        let add_tag_results = try_join_all(add_tag_futures).await.inspect_err(|err| {
          log::error!("{err}");
        })?;

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
              .map_err(|_err| anyhow::anyhow!("error"))
          })
          .collect();

        let add_path_results = try_join_all(add_path_futures)
          .await
          .inspect_err(|err| log::error!("{err}"))?;
        Ok((library, add_tag_results, add_path_results))
      })
    })
    .await;

  match result {
    Ok(result) => Ok(result),
    Err(_err) => Err(anyhow::anyhow!("")),
  }
}

/// delete library
pub async fn delete_library(lid: u64, db: Arc<DatabaseConnection>) -> Result<()> {
  let library = librarys::ActiveModel {
    lid: Set(lid),
    ..Default::default()
  };

  library
    .delete(db.as_ref())
    .await
    .map_err(|_err| anyhow::anyhow!("error"))?;
  Ok(())
}

pub async fn add_tag_for_library(
  lid: u64,
  tags: Vec<u64>,
  db: Arc<DatabaseConnection>,
) -> Result<Vec<librarys_related_tags::Model>> {
  let add_tag_futures: Vec<_> = tags
    .iter()
    .map(|x| async {
      let library_tag_relation = librarys_related_tags::ActiveModel {
        lid: Set(lid),
        tid: Set(*x),
        ..Default::default()
      };
      library_tag_relation
        .insert(db.as_ref())
        .await
        .map_err(|err| anyhow::anyhow!("error"))
    })
    .collect();
  let add_tags_result = try_join_all(add_tag_futures).await?;
  Ok(add_tags_result)
}

pub async fn get_library(lid: u64, db: Arc<DatabaseConnection>) -> Result<Option<librarys::Model>> {
  match librarys::Entity::find_by_id(lid).one(db.as_ref()).await {
    Ok(library) => Ok(library),
    Err(_err) => Err(anyhow::anyhow!("error")),
  }
}

pub async fn check_if_paths_overlap(paths: Vec<String>, _db: Arc<DatabaseConnection>) -> bool {
  true
}
