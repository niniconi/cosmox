use std::{str::FromStr, sync::Arc};

use futures::future::try_join_all;
use sea_orm::{
  ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, Iden, PaginatorTrait,
  QueryOrder, TransactionError, TransactionTrait,
};

use crate::{
  controller::{
    resource_controller::{ResourceError, ResourceQueryRequest},
    tag_controller::TagError,
  },
  entities::{resources, resources_related_tags, tags},
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
        let add_tag_futures = tags
          .iter()
          .map(|tid| async {
            let resource_tag_relation = resources_related_tags::ActiveModel {
              rid: Set(rid),
              tid: Set(*tid),
              ..Default::default()
            };
            resource_tag_relation.insert(db.as_ref()).await
          })
          .collect::<Vec<_>>();

        Ok(
          try_join_all(add_tag_futures)
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|_err| ResourceError::InternalError("Database error".to_string()))?,
        )
      })
    })
    .await;
  let result = result.map_err(|err| todo!())?;
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
    Err(err) => Err(ResourceError::InternalError("Database error".to_string())),
  }
}
