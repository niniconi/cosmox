use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use common::message::Pagination;
use cosmox_api::metadata::Metadata;
use cosmox_macros::page_helper;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    entities::{resources, resources_related_tags},
    get_db_connection,
};

/// Errors related to individual media file (resource) operations.
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("Resource '{0}' not found.")]
    NotFound(u64),

    #[error("Resource with URL '{0}' already exists.")]
    UrlConflict(String),

    #[error(
        "Parameter conflict: Cannot specify 'level' and 'min_level'/'max_level' at the same time."
    )]
    LevelParameterConflict,

    #[error("Parameter error: 'min_level' cannot be greater than 'max_level'.")]
    InvalidLevelRange,

    #[error("Failed to parse resource content: {0}")]
    ContentParseError(String),

    #[error("Resource '{0}' is too large; maximum size is {1} bytes.")]
    TooLarge(u64, u64),

    #[error("Resource '{0}' cannot be deleted due to dependencies.")]
    DeletionConflict(u64),

    #[error("Resource '{0}' is currently being processed.")]
    ProcessingConflict(u64),

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

#[derive(Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct ResourceAddRequest {
    pub name: String,
    pub lid: u64,
    pub description: Option<String>,
    pub level: u64,
}

#[derive(Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct ResourceDeleteRequest {
    pub rid: u64,
}

#[page_helper]
#[derive(Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct ResourceQueryRequest {
    lid: u64,
    level: Option<u64>,
    min_level: Option<u64>,
    max_level: Option<u64>,
}

#[derive(Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct ResourceAddTagRequest {
    pub tags: Vec<u64>,
}

/// query resource from database
pub async fn get_resource(rid: u64) -> Result<resources::Model, ResourceError> {
    let db = get_db_connection().await;
    get_resource_db(&db, rid).await
}

pub async fn get_resource_db(
    db: &DatabaseConnection,
    rid: u64,
) -> Result<resources::Model, ResourceError> {
    let resource = resources::Entity::find_by_id(rid)
        .one(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            ResourceError::InternalError(format!(
                "Database error: get resource {rid} failed: {err}"
            ))
        })?;

    match resource {
        Some(resource) => Ok(resource),
        None => Err(ResourceError::NotFound(rid)),
    }
}

pub async fn add_resource(payload: ResourceAddRequest) -> Result<u64, ResourceError> {
    let db = get_db_connection().await;
    add_resource_db(&db, payload).await
}

pub async fn add_resource_db(
    db: &DatabaseConnection,
    payload: ResourceAddRequest,
) -> Result<u64, ResourceError> {
    let current_datetime = Utc::now().naive_utc();
    let resource = resources::ActiveModel {
        name: Set(Some(payload.name.clone())),
        description: Set(payload.description),
        lid: Set(Some(payload.lid)),
        create_datetime: Set(current_datetime),
        last_update_datetime: Set(current_datetime),
        level: Set(payload.level),
        ..Default::default()
    };
    let resource = resource
        .insert(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            ResourceError::InternalError(format!(
                "Database error: insert resource '{}' in library {} failed: {err}",
                payload.name, payload.lid
            ))
        })?;
    Ok(resource.rid)
}
pub async fn add_resource_by_metadata(
    lid: u64,
    metadata: &Metadata<()>,
    level: u64,
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
        level: Set(level),
        ..Default::default()
    };

    let resource = resource
        .insert(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            ResourceError::InternalError(format!(
                "Database error: insert resource by metadata (lid={lid}) failed: {err}"
            ))
        })?;
    Ok(resource.rid)
}

/// delete resource from database
pub async fn delete_resource(rid: u64) -> Result<(), ResourceError> {
    let db = get_db_connection().await;
    delete_resource_db(&db, rid).await
}

pub async fn delete_resource_db(db: &DatabaseConnection, rid: u64) -> Result<(), ResourceError> {
    resources::Entity::delete_by_id(rid)
        .exec(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            ResourceError::InternalError(format!("Delete resource {rid} failed: {err}"))
        })?;
    Ok(())
}

/// add tags for resource
pub async fn add_tags_for_resource(
    rid: u64,
    tags: Vec<u64>,
) -> Result<Vec<resources_related_tags::Model>, ResourceError> {
    let db = get_db_connection().await;
    add_tags_for_resource_db(&db, rid, tags).await
}

pub async fn add_tags_for_resource_db(
    db: &DatabaseConnection,
    rid: u64,
    tags: Vec<u64>,
) -> Result<Vec<resources_related_tags::Model>, ResourceError> {
    let result = db
        .transaction::<_, Vec<resources_related_tags::Model>, ResourceError>(|txn| {
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
                    .exec(txn)
                    .await
                    .inspect_err(|err| log::error!("{err}"))
                    .map_err(|err| {
                        ResourceError::InternalError(format!(
                            "Database error: insert tags for resource {rid} failed: {err}"
                        ))
                    })
                    .map(|_| vec![]) // TODO solve return models
            })
        })
        .await;

    let result = result
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            ResourceError::InternalError(format!(
                "Database error: add tags for resource {rid} transaction failed: {err}"
            ))
        })?;
    Ok(result)
}

/// query resources from database
pub async fn query_resources(
    params: ResourceQueryRequest,
) -> Result<(Vec<resources::Model>, Pagination), ResourceError> {
    let db = get_db_connection().await;
    query_resources_db(&db, params).await
}

pub async fn query_resources_db(
    db: &DatabaseConnection,
    params: ResourceQueryRequest,
) -> Result<(Vec<resources::Model>, Pagination), ResourceError> {
    let mut select = resources::Entity::find().filter(resources::Column::Lid.eq(params.lid));
    let mut page = 0;

    match (params.level, params.min_level, params.max_level) {
        (Some(level), None, None) => {
            select = select.filter(resources::Column::Level.eq(level));
        }
        (None, Some(min_level), Some(max_level)) => {
            if min_level > max_level {
                return Err(ResourceError::InvalidLevelRange);
            }
            select = select.filter(resources::Column::Level.between(min_level, max_level));
        }
        (None, Some(min_level), None) => {
            select = select.filter(resources::Column::Level.gte(min_level));
        }
        (None, None, Some(max_level)) => {
            select = select.filter(resources::Column::Level.lte(max_level));
        }
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
            return Err(ResourceError::LevelParameterConflict);
        }
        _ => (),
    }

    if let Some(inner_page) = params.page {
        page = inner_page;
    }

    if let Some(sort) = &params.sort
        && let Ok(column) = resources::Column::from_str(sort)
    {
        select = select.order_by(column, sea_orm::Order::Asc);
    };

    let paginator = select.paginate(db, params.page_size);
    let total = paginator
        .num_items()
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| ResourceError::InternalError(format!("Count resources failed: {err}")))?;
    let pagination = Pagination::new(total, params.page_size, paginator.cur_page(), "");

    match paginator.fetch_page(page).await {
        Ok(result) => Ok((result, pagination)),
        Err(err) => {
            log::error!("{err}");
            Err(ResourceError::InternalError(format!(
                "Query resources failed: {err}"
            )))
        }
    }
}
