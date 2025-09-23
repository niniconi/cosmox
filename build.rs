use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::env;

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
  #[serde(rename = "server")]
  pub _ignore: Value,
  #[serde(rename = "database")]
  pub database: DatabaseConfiguration,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseConfiguration {
  pub host: String,
  pub port: u16,
  pub user: String,
  pub password: String,
  pub database: String,
}

fn main() {
  if env::var("ORM_GENERATE").is_ok() {
    let sql =
      weaver_orm_generator::load_sql_from_remote("192.168.1.254", 3306, "root", "123456", "media")
        .unwrap();
    weaver_orm_generator::generate("./src/entities", sql).unwrap();
  }
}
