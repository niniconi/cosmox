use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(
  Debug, Clone, PartialEq, Eq, Hash, FromRow, Serialize, Deserialize, ToSchema, DeriveEntityModel,
)]
#[sea_orm(table_name = "resources")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub rid: u64,
  pub name: Option<String>,
  pub description: Option<String>,
  pub lid: Option<u64>,
  #[serde(with = "chrono::naive::serde::ts_seconds")]
  pub create_datetime: NaiveDateTime,
  #[serde(with = "chrono::naive::serde::ts_seconds")]
  pub last_update_datetime: NaiveDateTime,
  pub metadata_parent_path: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
  fn def(&self) -> RelationDef {
    panic!("No Relation");
  }
}

impl ActiveModelBehavior for ActiveModel {}
