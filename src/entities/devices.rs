use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// devices
#[derive(
  Debug, Clone, PartialEq, Eq, Hash, FromRow, Serialize, Deserialize, ToSchema, DeriveEntityModel,
)]
#[sea_orm(table_name = "devices")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub did: u64,
  pub user_agent: Option<String>,
  pub login_by_uid: Option<u64>,
  #[serde(with = "chrono::naive::serde::ts_seconds")]
  pub last_login_datetime: NaiveDateTime,
  pub last_login_ip: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
  fn def(&self) -> RelationDef {
    panic!("No Relation");
  }
}

impl ActiveModelBehavior for ActiveModel {}
