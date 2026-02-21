use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use cosmox_api::metadata::Metadata;
use sea_orm::{
  ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder,
  TransactionTrait,
};

use crate::{
  controller::resource_controller::{ResourceError, ResourceQueryRequest},
  entities::{resources, resources_related_tags},
  utils::message::Pagination,
};

/// query resource from database
pub async fn get_resource(
  rid: u64,
  db: Arc<DatabaseConnection>,
) -> Result<resources::Model, ResourceError> {
  let resource = resources::Entity::find_by_id(rid)
    .one(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| ResourceError::InternalError("Database error".to_string()))?;

  match resource {
    Some(resource) => Ok(resource),
    None => Err(ResourceError::NotFound(rid)),
  }
}

pub async fn add_resource(db: Arc<DatabaseConnection>) -> Result<(), ResourceError> {
  todo!()
}
pub async fn add_resource_by_metadata(
  lid: u64,
  metadata: &Metadata<()>,
  db: Arc<DatabaseConnection>,
) -> Result<u64, ResourceError> {
  let current_datetime = Utc::now().naive_utc();
  let resource = resources::ActiveModel {
    name: Set(Some(metadata.name.clone())),
    description: Set(Some(metadata.description.clone())),
    create_datetime: Set(current_datetime),
    last_update_datetime: Set(current_datetime),
    lid: Set(Some(lid)),
    cover: Set(metadata.cover_file_map_id),
    ..Default::default()
  };

  let resource = resource
    .insert(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| ResourceError::InternalError("Database error".to_string()))?;
  Ok(resource.rid)
}

/// delete resource from database
pub async fn delete_resource(rid: u64, db: Arc<DatabaseConnection>) -> Result<(), ResourceError> {
  resources::Entity::delete_by_id(rid)
    .exec(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| {
      ResourceError::InternalError(format!("Delete resource {rid} failed by Database error"))
    })?;
  Ok(())
}

/// add tags for resource
pub async fn add_tags_for_resource(
  rid: u64,
  tags: Vec<u64>,
  db: Arc<DatabaseConnection>,
) -> Result<Vec<resources_related_tags::Model>, ResourceError> {
  let result = db
    .clone()
    .transaction::<_, Vec<resources_related_tags::Model>, ResourceError>(|_txn| {
      Box::pin(async move {
        let resource_tag_relations = tags
          .iter()
          .map(|tid| resources_related_tags::ActiveModel {
            rid: Set(rid),
            tid: Set(*tid),
            ..Default::default()
          })
          .collect::<Vec<_>>();

        resources_related_tags::Entity::insert_many(resource_tag_relations)
          .exec(db.as_ref())
          .await
          .inspect_err(|err| log::error!("{err}"))
          .map_err(|_err| ResourceError::InternalError("Database error".to_string()))
          .map(|_| vec![]) // TODO solve return models
      })
    })
    .await;

  let result = result
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| ResourceError::InternalError("Database error".to_string()))?;
  Ok(result)
}

/// query resources from database
pub async fn query_resources(
  params: ResourceQueryRequest,
  db: Arc<DatabaseConnection>,
) -> Result<(Vec<resources::Model>, Pagination), ResourceError> {
  let mut select = resources::Entity::find();
  let mut page = 0;

  if let Some(inner_page) = params.page {
    page = inner_page;
  }

  if let Some(sort) = &params.sort
    && let Ok(column) = resources::Column::from_str(sort)
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
    Err(_err) => Err(ResourceError::InternalError("Database error".to_string())),
  }
}
