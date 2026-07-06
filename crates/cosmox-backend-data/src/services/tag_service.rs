use std::str::FromStr;

use chrono::Utc;
use common::message::Pagination;
use cosmox_macros::page_helper;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait,
    PaginatorTrait, QueryFilter, QueryOrder, SqlErr,
};
use serde::{Deserialize, Serialize};

use crate::{
    define::{Tag, TagGroups},
    entities::{tag_groups, tags},
    get_db_connection,
};

/// Errors related to tag management.
#[derive(Debug, thiserror::Error)]
pub enum TagError {
    #[error("Tag '{0}' not found.")]
    NotFound(u64),

    #[error("Not authorized to manage tags.")]
    Unauthorized,

    #[error("Tag '{0}' already exists.")]
    AlreadyExists(String),

    #[error("Tag name '{0}' is invalid: {1}")]
    InvalidName(String, String),

    #[error("Maximum number of tags ({0}) reached for resource '{1}'.")]
    MaxTagsExceeded(u32, u64),

    #[error("Tag '{0}' is protected and cannot be modified or deleted.")]
    ProtectedTag(String),

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

#[page_helper]
#[derive(Debug, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct TagQueryRequest {
    pub tid: Option<u64>,
}

#[page_helper]
#[derive(Debug, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct TagGroupQueryRequest {
    pub tgid: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct TagAddRequest {
    pub label: String,
    pub tgid: u64,
}

#[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct TagGroupAddRequest {
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct TagGroupDeleteRequest {
    pub tgid: u64,
}

#[derive(Debug, Serialize)]
pub struct TagCatalogEntry {
    pub group: TagGroups,
    pub tags: Vec<Tag>,
}

pub async fn add_tag_group(label: String) -> Result<u64, TagError> {
    let db = get_db_connection().await;
    add_tag_group_db(&db, label).await
}

pub async fn add_tag_group_db(db: &DatabaseConnection, label: String) -> Result<u64, TagError> {
    let current_navie_datetime = Utc::now().naive_utc();
    let tag_group = tag_groups::ActiveModel {
        text: Set(label.clone()),
        create_datetime: Set(current_navie_datetime),
        ..Default::default()
    };
    let tag_group = tag_group.insert(db).await.map_err(|err| {
        if let Some(sql_err) = err.sql_err()
            && let SqlErr::UniqueConstraintViolation(_) = sql_err
        // TODO check field
        {
            TagError::AlreadyExists(format!("group {label}"))
        } else {
            log::error!("Failed to insert tag group '{label}': {err}");
            TagError::InternalError(format!(
                "Database error: insert tag group '{label}' failed: {err}"
            ))
        }
    })?;
    Ok(tag_group.tgid)
}

pub async fn get_tag_group(tgid: u64) -> Result<TagGroups, TagError> {
    let db = get_db_connection().await;
    get_tag_group_db(&db, tgid).await
}

pub async fn get_tag_group_db(db: &DatabaseConnection, tgid: u64) -> Result<TagGroups, TagError> {
    let tag_group = tag_groups::Entity::find_by_id(tgid)
        .one(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            TagError::InternalError(format!(
                "Database error: get tag group {tgid} failed: {err}"
            ))
        })?;
    match tag_group {
        Some(group) => Ok(group),
        None => Err(TagError::NotFound(tgid)),
    }
}

pub async fn get_tag_group_by_label(label: String) -> Result<Option<TagGroups>, TagError> {
    let db = get_db_connection().await;
    get_tag_group_by_label_db(&db, label).await
}

pub async fn get_tag_group_by_label_db(
    db: &DatabaseConnection,
    label: String,
) -> Result<Option<TagGroups>, TagError> {
    let tag_group = tag_groups::Entity::find()
        .filter(tag_groups::Column::Text.eq(label.clone()))
        .one(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            TagError::InternalError(format!(
                "Database error: get tag group by label '{label}' failed: {err}"
            ))
        })?;
    Ok(tag_group)
}

pub async fn get_tag(tid: u64) -> Result<Tag, TagError> {
    let db = get_db_connection().await;
    get_tag_db(&db, tid).await
}

pub async fn get_tag_db(db: &DatabaseConnection, tid: u64) -> Result<Tag, TagError> {
    let tag = tags::Entity::find_by_id(tid)
        .one(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            TagError::InternalError(format!("Database error: get tag {tid} failed: {err}"))
        })?;
    match tag {
        Some(tag) => Ok(tag),
        None => Err(TagError::NotFound(tid)),
    }
}

pub async fn get_tag_by_label(label: String) -> Result<Option<Tag>, TagError> {
    let db = get_db_connection().await;
    get_tag_by_label_db(&db, label).await
}

pub async fn get_tag_by_label_db(
    db: &DatabaseConnection,
    label: String,
) -> Result<Option<Tag>, TagError> {
    let tag = tags::Entity::find()
        .filter(tags::Column::Text.eq(label.clone()))
        .one(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            TagError::InternalError(format!(
                "Database error: get tag by label '{label}' failed: {err}"
            ))
        })?;
    Ok(tag)
}

pub async fn add_tag(label: String, tgid: u64) -> Result<u64, TagError> {
    let db = get_db_connection().await;
    add_tag_db(&db, label, tgid).await
}

pub async fn add_tag_db(
    db: &DatabaseConnection,
    label: String,
    tgid: u64,
) -> Result<u64, TagError> {
    let current_navie_datetime = Utc::now().naive_utc();
    let tag = tags::ActiveModel {
        text: Set(label.clone()),
        tgid: Set(tgid),
        create_datetime: Set(current_navie_datetime),
        ..Default::default()
    };
    let tag = tag.insert(db).await.map_err(|err| {
        if let Some(sql_err) = err.sql_err()
            && let SqlErr::UniqueConstraintViolation(_) = sql_err
        // TODO check field
        {
            TagError::AlreadyExists(format!("{tgid}:{label}"))
        } else {
            log::error!("Failed to insert tag '{label}' in group {tgid}: {err}");
            TagError::InternalError(format!(
                "Database error: insert tag '{label}' in group {tgid} failed: {err}"
            ))
        }
    })?;
    Ok(tag.tid)
}

pub async fn delete_tag(tid: u64) -> Result<(), TagError> {
    let db = get_db_connection().await;
    delete_tag_db(&db, tid).await
}

pub async fn delete_tag_db(db: &DatabaseConnection, tid: u64) -> Result<(), TagError> {
    let _result = tags::Entity::delete_by_id(tid)
        .exec(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            TagError::InternalError(format!("Database error: delete tag {tid} failed: {err}"))
        })?;
    Ok(())
}

pub async fn delete_tag_group(tgid: u64) -> Result<(), TagError> {
    let db = get_db_connection().await;
    delete_tag_group_db(&db, tgid).await
}

pub async fn delete_tag_group_db(db: &DatabaseConnection, tgid: u64) -> Result<(), TagError> {
    tag_groups::Entity::delete_by_id(tgid)
        .exec(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            TagError::InternalError(format!(
                "Database error: delete tag group {tgid} failed: {err}"
            ))
        })?;
    Ok(())
}

pub async fn query_tag_group(
    params: TagGroupQueryRequest,
) -> Result<(Vec<TagGroups>, Pagination), TagError> {
    let db = get_db_connection().await;
    query_tag_group_db(&db, params).await
}

pub async fn query_tag_group_db(
    db: &DatabaseConnection,
    params: TagGroupQueryRequest,
) -> Result<(Vec<TagGroups>, Pagination), TagError> {
    let mut select = tag_groups::Entity::find();
    let mut page = 0;

    if let Some(inner_page) = params.page {
        page = inner_page;
    }

    if let Some(sort) = &params.sort
        && let Ok(column) = tag_groups::Column::from_str(sort)
    {
        select = select.order_by(column, sea_orm::Order::Asc);
    };

    let paginator = select.paginate(db, params.page_size);
    let total = paginator
        .num_items()
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            TagError::InternalError(format!("Database error: count tag groups failed: {err}"))
        })?;
    let pagination = Pagination::new(total, params.page_size, paginator.cur_page(), "");

    match paginator.fetch_page(page).await {
        Ok(result) => Ok((result, pagination)),
        Err(err) => {
            log::error!("Database error: query tag groups page {page} failed: {err}",);
            Err(TagError::InternalError(format!(
                "Database error: query tag groups failed: {err}"
            )))
        }
    }
}

pub async fn query_catalog() -> Result<Vec<TagCatalogEntry>, TagError> {
    let db = get_db_connection().await;
    query_catalog_db(&db).await
}

pub async fn query_catalog_db(db: &DatabaseConnection) -> Result<Vec<TagCatalogEntry>, TagError> {
    let groups = tag_groups::Entity::find()
        .all(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            TagError::InternalError(format!(
                "Database error: load tag groups for catalog failed: {err}"
            ))
        })?;

    let mut catalog = Vec::new();
    for group in groups {
        let group_tags = group
            .find_related(tags::Entity)
            .all(db)
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| {
                TagError::InternalError(format!(
                    "Database error: load tags for tag group {} failed: {err}",
                    group.tgid
                ))
            })?;
        catalog.push(TagCatalogEntry {
            group,
            tags: group_tags,
        });
    }
    Ok(catalog)
}

pub async fn query_tag(params: TagQueryRequest) -> Result<(Vec<Tag>, Pagination), TagError> {
    let db = get_db_connection().await;
    query_tag_db(&db, params).await
}

pub async fn query_tag_db(
    db: &DatabaseConnection,
    params: TagQueryRequest,
) -> Result<(Vec<Tag>, Pagination), TagError> {
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

    let paginator = select.paginate(db, params.page_size);
    let total = paginator
        .num_items()
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            TagError::InternalError(format!("Database error: count tags failed: {err}"))
        })?;
    let pagination = Pagination::new(total, params.page_size, paginator.cur_page(), "");

    match paginator.fetch_page(page).await {
        Ok(result) => Ok((result, pagination)),
        Err(err) => {
            log::error!("Database error: query tags page {page} failed: {err}");
            Err(TagError::InternalError(format!(
                "Database error: query tags failed: {err}"
            )))
        }
    }
}
