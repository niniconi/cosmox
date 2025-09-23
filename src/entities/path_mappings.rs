use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(
  Debug, Clone, PartialEq, Eq, Hash, FromRow, Serialize, Deserialize, ToSchema, DeriveEntityModel,
)]
#[sea_orm(table_name = "path_mappings")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub pmid: u64,
  pub path: Option<String>,
  pub content_type: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
  fn def(&self) -> RelationDef {
    panic!("No Relation");
  }
}

impl ActiveModelBehavior for ActiveModel {}
