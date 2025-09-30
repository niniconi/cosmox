use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(
  Debug, Clone, PartialEq, Eq, Hash, FromRow, Serialize, Deserialize, ToSchema, DeriveEntityModel,
)]
#[sea_orm(table_name = "resources_related_tags")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub rrtid: u64,
  pub rid: u64,
  pub tid: u64,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
  fn def(&self) -> RelationDef {
    panic!("No Relation");
  }
}

impl ActiveModelBehavior for ActiveModel {}
