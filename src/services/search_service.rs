use std::{str::FromStr, sync::Arc};

use sea_orm::{
  ColumnTrait, Condition, DatabaseConnection, EntityTrait, JoinType, PaginatorTrait, QueryFilter,
  QueryOrder, QuerySelect, RelationTrait,
};

use crate::{
  controller::search_controller::{SearchError, SearchRequest},
  entities::{resources, resources_related_tags, tags},
  utils::message::Pagination,
};

pub async fn search(
  params: SearchRequest,
  db: Arc<DatabaseConnection>,
) -> Result<(Vec<resources::Model>, Pagination), SearchError> {
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

  let pagination = Pagination::new(
    paginator.num_items().await.unwrap(),
    params.page_size,
    paginator.cur_page(),
    "",
  );

  match paginator.fetch_page(page).await {
    Ok(result) => Ok((result, pagination)),
    Err(_) => Err(SearchError::InternalError("Database error".to_string())),
  }
}
