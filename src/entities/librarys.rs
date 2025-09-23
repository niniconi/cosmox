use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(
  Debug, Clone, PartialEq, Eq, Hash, FromRow, Serialize, Deserialize, ToSchema, DeriveEntityModel,
)]
#[sea_orm(table_name = "librarys")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub lid: u64,
  pub name: Option<String>,
  pub description: Option<String>,
  #[serde(with = "chrono::naive::serde::ts_seconds")]
  pub create_datetime: NaiveDateTime,
  #[serde(with = "chrono::naive::serde::ts_seconds")]
  pub last_update_datetime: NaiveDateTime,
  pub r#type: Option<u64>,
  pub create_by_uid: u64,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
  fn def(&self) -> RelationDef {
    panic!("No Relation");
  }
}

impl ActiveModelBehavior for ActiveModel {}
