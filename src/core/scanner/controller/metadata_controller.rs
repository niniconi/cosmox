use std::{fs::File, io::BufReader, sync::Arc};

use actix_web::{HttpResponse, Responder, get, web};
use cosmox_api::metadata::Metadata;
use cosmox_macros::auto_webapi_doc;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::into_message;

#[derive(IntoParams, Deserialize)]
pub struct MetadataQueryRequest {
  pub root_node: u64,
  pub depth: usize,
}

#[auto_webapi_doc]
#[get("{rid}")]
pub async fn get(rid: web::Path<u64>, db: web::Data<DatabaseConnection>) -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented add api")
}

#[auto_webapi_doc]
#[get("query")]
pub async fn query(
  query: web::Query<MetadataQueryRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented add api")
}
