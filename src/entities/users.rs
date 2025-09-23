use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// user table
#[derive(
  Debug, Clone, PartialEq, Eq, Hash, FromRow, Serialize, Deserialize, ToSchema, DeriveEntityModel,
)]
#[sea_orm(table_name = "users")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub uid: u64,
  pub password: String,
  pub username: String,
  pub avatar: Option<u64>,
  pub nickname: Option<String>,
  #[serde(with = "chrono::naive::serde::ts_seconds")]
  pub create_datetime: NaiveDateTime,
  #[serde(with = "chrono::naive::serde::ts_seconds")]
  pub last_update_datetime: NaiveDateTime,
  pub email: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
  fn def(&self) -> RelationDef {
    panic!("No Relation");
  }
}

impl ActiveModelBehavior for ActiveModel {}
