use std::str::FromStr;

use chrono::NaiveDateTime;
use common::message::Pagination;
use cosmox_macros::page_helper;
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, JoinType, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait,
};
use serde::Deserialize;

use crate::{
    entities::{resources, resources_related_tags, tags},
    get_db_connection,
};

/// Errors related to search.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Not authorized to manage tags.")]
    Unauthorized,

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

#[page_helper]
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub keyword: String,
    pub tags: Option<Vec<String>>,
    pub lid: Option<u64>,
    pub before_create_datetime: Option<NaiveDateTime>,
    pub after_create_datetime: Option<NaiveDateTime>,
    pub before_last_update_datetime: Option<NaiveDateTime>,
    pub after_last_update_datetime: Option<NaiveDateTime>,
}

pub async fn search(
    params: SearchRequest,
) -> Result<(Vec<resources::Model>, Pagination), SearchError> {
    let db = get_db_connection().await;
    let mut select_resource = resources::Entity::find();
    let mut page = 0;
    let search_fmt = format!("%{}%", params.keyword);

    let mut cond = Condition::any()
        .add(resources::Column::Description.like(&search_fmt))
        .add(resources::Column::Name.like(&search_fmt));
    cond = Condition::all().add(cond);

    if let Some(lid) = params.lid {
        cond = cond.add(resources::Column::Lid.eq(lid));
    }

    if let Some(tags) = params.tags {
        cond = cond.add(tags::Column::Text.is_in(tags));
        select_resource = select_resource
            .join(
                JoinType::InnerJoin,
                resources::Relation::ResourcesRelatedTags.def(),
            )
            .join(
                JoinType::InnerJoin,
                resources_related_tags::Relation::Tags.def(),
            );
    }

    if let Some(before) = params.before_create_datetime {
        cond = cond.add(resources::Column::CreateDatetime.lte(before));
    }

    if let Some(after) = params.after_create_datetime {
        cond = cond.add(resources::Column::CreateDatetime.gte(after));
    }

    if let Some(before) = params.before_last_update_datetime {
        cond = cond.add(resources::Column::LastUpdateDatetime.lte(before));
    }

    if let Some(after) = params.after_last_update_datetime {
        cond = cond.add(resources::Column::LastUpdateDatetime.gte(after));
    }

    if let Some(inner_page) = params.page {
        page = inner_page;
    }

    if let Some(sort) = &params.sort
        && let Ok(column) = resources::Column::from_str(sort)
    {
        select_resource = select_resource.order_by(column, sea_orm::Order::Asc);
    };

    let paginator = select_resource
        .filter(cond)
        .distinct()
        .paginate(db.as_ref(), params.page_size);

    let total = paginator
        .num_items()
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| SearchError::InternalError(format!("Count search results failed: {err}")))?;
    let pagination = Pagination::new(total, params.page_size, paginator.cur_page(), "");

    match paginator.fetch_page(page).await {
        Ok(result) => Ok((result, pagination)),
        Err(err) => {
            log::error!("{err}");
            Err(SearchError::InternalError(format!(
                "Search query failed: {err}"
            )))
        }
    }
}
