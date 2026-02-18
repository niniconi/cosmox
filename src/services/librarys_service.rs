use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use futures::future::try_join_all;
use sea_orm::{
  ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder,
  TransactionTrait,
};

use crate::{
  controller::library_controller::{LibraryAddRequest, LibraryError, LibraryQueryRequest},
  entities::{library_paths, librarys, librarys_related_tags},
  user::security::auth_middleware::RequestUser,
  utils::message::Pagination,
};

/// create library with tags and paths
/// # Arguments
/// - `Arc<LibraryAddRequest>`
/// - `Arc<DatabaseConnection>`
pub async fn create_library_with_tags_and_paths(
  payload: Arc<LibraryAddRequest>,
  db: Arc<DatabaseConnection>,
  req_user: RequestUser,
) -> Result<
  (
    librarys::Model,
    Vec<librarys_related_tags::Model>,
    Vec<library_paths::Model>,
  ),
  LibraryError,
> {
  let uid = match req_user.uid {
    Some(uid) => uid,
    None => {
      log::error!("Expect `uid` at {req_user:?}");
      return Err(LibraryError::InternalError(
        "Expected a uid, got None".to_string(),
      ));
    }
  };
  // check

  // insert into database
  let result = db
    .clone()
    .transaction::<_, (librarys::Model, Vec<_>, Vec<_>), LibraryError>(|_txn| {
      Box::pin(async move {
        let current_datetime = Utc::now().naive_utc();
        let library = librarys::ActiveModel {
          name: Set(Some(payload.name.clone())),
          description: Set(payload.description.clone()),
          create_datetime: Set(current_datetime),
          last_update_datetime: Set(current_datetime),
          create_by_uid: Set(uid),
          ..Default::default()
        };
        let library = library
          .insert(db.as_ref())
          .await
          .inspect_err(|err| log::error!("{err}"))
          .map_err(|_err| LibraryError::InternalError("Database error".to_string()))?;

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
              .inspect_err(|err| log::error!("{err}"))
              .map_err(|_err| LibraryError::InternalError("Database error".to_string()))
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
              .insert(db.as_ref())
              .await
              .inspect_err(|err| log::error!("{err}"))
              .map_err(|_err| LibraryError::InternalError("Database error".to_string()))
          })
          .collect();

        let add_path_results = try_join_all(add_path_futures).await?;
        Ok((library, add_tag_results, add_path_results))
      })
    })
    .await;

  let result = result.map_err(|_err| todo!())?;
  Ok(result)
}

/// delete library
pub async fn delete_library(lid: u64, db: Arc<DatabaseConnection>) -> Result<(), LibraryError> {
  librarys::Entity::delete_by_id(lid)
    .exec(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| LibraryError::InternalError("Database error".to_string()))?;
  Ok(())
}

pub async fn add_tags_for_library(
  lid: u64,
  tags: Vec<u64>,
  db: Arc<DatabaseConnection>,
) -> Result<Vec<librarys_related_tags::Model>, LibraryError> {
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
        .inspect_err(|err| log::error!("{err}"))
    })
    .collect();

  let add_tags_result = try_join_all(add_tag_futures)
    .await
    .map_err(|_err| LibraryError::InternalError("Database error".to_string()))?;
  Ok(add_tags_result)
}

pub async fn get_library(
  lid: u64,
  db: Arc<DatabaseConnection>,
) -> Result<librarys::Model, LibraryError> {
  let library = librarys::Entity::find_by_id(lid)
    .one(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| LibraryError::InternalError("Database error".to_string()))?;

  match library {
    Some(library) => Ok(library),
    None => Err(LibraryError::NotFound(lid)),
  }
}

pub async fn query_libraies(
  params: LibraryQueryRequest,
  db: Arc<DatabaseConnection>,
) -> Result<(Vec<librarys::Model>, Pagination), LibraryError> {
  let mut select = librarys::Entity::find();
  let mut page = 0;

  if let Some(inner_page) = params.page {
    page = inner_page;
  }

  if let Some(sort) = &params.sort
    && let Ok(column) = librarys::Column::from_str(sort)
  {
    select = select.order_by(column, sea_orm::Order::Asc);
  };

  let paginator = select.paginate(db.as_ref(), params.page_size);
  let pagination = Pagination::new(
    paginator.num_items().await.unwrap(),
    params.page_size,
    paginator.cur_page(),
    "",
  );

  match paginator.fetch_page(page).await {
    Ok(result) => Ok((result, pagination)),
    Err(err) => Err(LibraryError::InternalError("Database error".to_string())),
  }
}

pub async fn check_if_paths_overlap(paths: Vec<String>, _db: Arc<DatabaseConnection>) -> bool {
  true
}
