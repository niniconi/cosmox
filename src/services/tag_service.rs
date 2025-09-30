use std::{str::FromStr, sync::Arc};

use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder};

use crate::{
  controller::tag_controller::{TagError, TagQueryRequest},
  entities::tags,
  utils::message::Pagination,
};

pub async fn get_tag_group(_tgid: u64, _db: Arc<DatabaseConnection>) {
  todo!()
}

pub async fn get_tag(tid: u64, db: Arc<DatabaseConnection>) -> Result<tags::Model, TagError> {
  let tag = tags::Entity::find_by_id(tid)
    .one(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|err| TagError::InternalError("Database error".to_string()))?;

  match tag {
    Some(tag) => Ok(tag),
    None => Err(TagError::NotFound(tid)),
  }
}

pub async fn add_tag(tag: tags::Model, db: Arc<DatabaseConnection>) -> Result<(), TagError> {
  let tag = tags::ActiveModel::from(tag);
  let tag = tag
    .insert(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| TagError::InternalError(format!("Database error")))?;
  Ok(())
}

pub async fn delete_tag(tid: u64, db: Arc<DatabaseConnection>) -> Result<(), TagError> {
  let _result = tags::Entity::delete_by_id(tid)
    .exec(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| TagError::InternalError(format!("Database error")))?;
  Ok(())
}

pub async fn query_tag(
  params: TagQueryRequest,
  db: Arc<DatabaseConnection>,
) -> Result<(Vec<tags::Model>, Pagination), TagError> {
  let mut select = tags::Entity::find();
  let mut page = 0;

  if let Some(inner_page) = params.page {
    page = inner_page;
  }

  if let Some(sort) = &params.sort
    && let Ok(column) = tags::Column::from_str(sort)
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
    Err(err) => Err(TagError::InternalError(format!("Database error"))),
  }
}
