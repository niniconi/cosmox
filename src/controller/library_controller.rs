use std::sync::Arc;

use actix_web::{HttpResponse, Responder, delete, get, post, web};
use cosmox_macros::{ActixWebError, auto_webapi_doc};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{entities::types, services::librarys_service, utils::message::Message};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LibraryAddRequest {
  pub name: String,
  pub description: Option<String>,

  pub r#type: u64,

  pub tags: Vec<u64>,
  pub library_paths: Vec<String>,
}

/// Errors related to library (collection of media files) operations.
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum LibraryError {
  #[error("Library '{0}' not found.")]
  #[code(404)]
  NotFound(u64),

  #[error("User '{0}' does not have access to library '{1}'.")]
  #[code(403)]
  Unauthorized(u64, u64),

  #[error("Library with name '{0}' already exists for user '{1}'.")]
  #[code(409)]
  NameConflict(String, u64),

  #[error("Library '{0}' is empty, cannot perform this operation.")]
  #[code(400)]
  EmptyLibrary(u64),

  #[error("Failed to update library metadata for '{0}': {1}")]
  #[code(500)]
  MetadataUpdateFailed(u64, String),

  #[error("Library '{0}' cannot be deleted due to associated resources.")]
  #[code(409)]
  DeletionConflict(u64),

  #[error("Exceeded maximum number of libraries allowed for user '{0}'.")]
  #[code(400)]
  MaxLibrariesExceeded(u64),

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

#[auto_webapi_doc]
#[get("{id}")]
pub async fn get(param: web::Path<u64>, db: web::Data<DatabaseConnection>) -> impl Responder {
  let result = librarys_service::get_library(param.into_inner(), db.into_inner()).await;
  HttpResponse::Ok().json(Message::ok(result.unwrap()))
}

#[auto_webapi_doc]
#[get("list")]
pub async fn list() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented list api")
}

#[auto_webapi_doc]
#[post("modify")]
pub async fn modify() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented modify api")
}

/// add library
///
/// add library with tags and path in disk.
#[auto_webapi_doc]
#[post("add")]
pub async fn add(
  body: web::Json<LibraryAddRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  let result = librarys_service::create_library_with_tags_and_paths(
    Arc::new(body.into_inner()),
    db.into_inner(),
  )
  .await;

  match result {
    Ok(library) => HttpResponse::Ok().json(Message::ok(Some(library))),
    Err(err) => {
      log::error!("{err}");
      HttpResponse::Ok().body("")
    }
  }
}

/// delete library
///
/// delete the entity in database table library.
/// delete the metadata information in disk (Option)
#[auto_webapi_doc]
#[delete("delete")]
pub async fn delete() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented delete api")
}

/// Returns all selectable Types
#[auto_webapi_doc]
#[get("type/all")]
pub async fn get_all_type(db: web::Data<DatabaseConnection>) -> impl Responder {
  match types::Entity::find().all(db.as_ref()).await {
    Ok(types) => HttpResponse::Ok().json(Message::ok(Some(types))),
    Err(_err) => {
      todo!()
    }
  }
}
