use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use sea_orm::{
  ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
  QueryFilter, QueryOrder, SqlErr,
};

use crate::{
  controller::tag_controller::{TagError, TagQueryRequest},
  entities::{tag_groups, tags},
  utils::message::Pagination,
};

pub async fn add_tag_group(label: String, db: Arc<DatabaseConnection>) -> Result<u64, TagError> {
  let current_navie_datetime = Utc::now().naive_utc();
  let tag_group = tag_groups::ActiveModel {
    text: Set(label.clone()),
    create_datetime: Set(current_navie_datetime),
    ..Default::default()
  };
  let tag_group = tag_group.insert(db.as_ref()).await.map_err(|err| {
    if let Some(sql_err) = err.sql_err()
      && let SqlErr::UniqueConstraintViolation(message) = sql_err
    // TODO check field
    {
      TagError::AlreadyExists(format!("group {label}"))
    } else {
      log::error!("{err}");
      TagError::InternalError("Database error".to_string())
    }
  })?;
  Ok(tag_group.tgid)
}

pub async fn get_tag_group(
  tgid: u64,
  db: Arc<DatabaseConnection>,
) -> Result<tag_groups::Model, TagError> {
  todo!();
}
pub async fn get_tag_group_by_label(
  label: String,
  db: Arc<DatabaseConnection>,
) -> Result<Option<tag_groups::Model>, TagError> {
  let tag_group = tag_groups::Entity::find()
    .filter(tag_groups::Column::Text.eq(label))
    .one(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_| TagError::InternalError("Database error".to_string()))?;

  Ok(tag_group)
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

pub async fn get_tag_by_label(
  label: String,
  db: Arc<DatabaseConnection>,
) -> Result<Option<tags::Model>, TagError> {
  let tag = tags::Entity::find()
    .filter(tags::Column::Text.eq(label))
    .one(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_| TagError::InternalError("Database error".to_string()))?;
  Ok(tag)
}

pub async fn add_tag(
  label: String,
  tgid: u64,
  db: Arc<DatabaseConnection>,
) -> Result<u64, TagError> {
  let current_navie_datetime = Utc::now().naive_utc();
  let tag = tags::ActiveModel {
    text: Set(label.clone()),
    tgid: Set(tgid),
    create_datetime: Set(current_navie_datetime),
    ..Default::default()
  };
  let tag = tag.insert(db.as_ref()).await.map_err(|err| {
    if let Some(sql_err) = err.sql_err()
      && let SqlErr::UniqueConstraintViolation(message) = sql_err
    // TODO check field
    {
      TagError::AlreadyExists(format!("{tgid}:{label}"))
    } else {
      log::error!("{err}");
      TagError::InternalError("Database error".to_string())
    }
  })?;
  Ok(tag.tid)
}

pub async fn delete_tag(tid: u64, db: Arc<DatabaseConnection>) -> Result<(), TagError> {
  let _result = tags::Entity::delete_by_id(tid)
    .exec(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| TagError::InternalError("Database error".to_string()))?;
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
    Err(err) => Err(TagError::InternalError("Database error".to_string())),
  }
}
