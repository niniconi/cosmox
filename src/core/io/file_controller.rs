use actix_web::{HttpResponse, Responder, get, post, web};
use cosmox_macros::{ActixWebError, auto_webapi_doc};
use sea_orm::{DatabaseConnection, EntityTrait};
// use futures_util::TryStreamExt;
// use tokio_util::codec::{BytesCodec, FramedRead};

use crate::entities::path_mappings;
// use futures::StreamExt;

/// Errors related to file operations (upload, download, management).
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum FileError {
  #[error("File '{0}' not found.")]
  #[code(404)]
  NotFound(String),

  #[error("Not authorized to perform file operation on '{0}'.")]
  #[code(403)]
  Unauthorized(String),

  #[error("File '{0}' already exists.")]
  #[code(409)]
  AlreadyExists(String),

  #[error("File upload failed: {0}")]
  #[code(500)]
  UploadFailed(String),

  #[error("File download failed: {0}")]
  #[code(500)]
  DownloadFailed(String),

  #[error("File '{0}' is too large; maximum size is {1} bytes.")]
  #[code(413)]
  TooLarge(String, u64),

  #[error("Invalid file type: {0}")]
  #[code(400)]
  InvalidFileType(String),

  #[error("Disk space exhausted: {0}")]
  #[code(507)]
  InsufficientStorage(String),

  #[error("File operation timed out for '{0}'.")]
  #[code(504)]
  // Gateway Timeout if waiting for external storage, or Request Timeout if internal file processing took too long
  OperationTimeout(String),

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

#[auto_webapi_doc]
#[post("push")]
pub async fn push() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented push api")

  // async fn upload_file(mut payload: Multipart) -> impl Responder {
  // while let Some(mut field) = payload.try_next().await.unwrap() {
  //     let content_disposition = field.content_disposition();
  //     let field_name = content_disposition
  //         .get_name()
  //         .unwrap_or("unknown_field");

  //     if let Some(filename) = content_disposition.get_filename() {
  //         let filepath = format!("./temp/{}", filename);
  //         println!("Saving file to: {}", filepath);

  //         let mut file = match tokio::fs::File::create(&filepath).await {
  //             Ok(f) => f,
  //             Err(e) => {
  //                 eprintln!("Failed to create file: {}", e);
  //                 return HttpResponse::InternalServerError().body(format!("Failed to create file: {}", e));
  //             }
  //         };

  //         while let Some(chunk) = field.try_next().await.unwrap() {
  //             if let Err(e) = file.write_all(&chunk).await {
  //                 eprintln!("Failed to write to file: {}", e);
  //                 return HttpResponse::InternalServerError().body(format!("Failed to write to file: {}", e));
  //             }
  //         }
  //         println!("File '{}' saved successfully.", filename);
  //     } else {
  //         let mut bytes = web::BytesMut::new();
  //         while let Some(chunk) = field.try_next().await.unwrap() {
  //             bytes.extend_from_slice(&chunk);
  //         }
  //         println!("Field '{}': {:?}", field_name, String::from_utf8_lossy(&bytes));
  //     }
  // }
}
#[auto_webapi_doc]
#[get("{id}/pull")]
pub async fn pull(file_id: web::Path<u64>, conn: web::Data<DatabaseConnection>) -> impl Responder {
  let path_mapping = path_mappings::Entity::find_by_id(file_id.into_inner())
    .one(conn.as_ref())
    .await
    .unwrap();
  println!("{path_mapping:#?}");
  HttpResponse::NotImplemented().body("Not implemented pull api")

  // pub async fn pull() -> impl Responder {
  // HttpResponse::Ok().body("")
  // }

  // let filename = file_id.into_inner();
  // let filepath = format!("./static/{}", filename);

  // try to open file
  // let file = match File::open(&filepath).await {
  // Ok(f) => f,
  // Err(e) => {
  // eprintln!("Failed to open file '{}': {}", filepath, e);
  // return HttpResponse::NotFound().body(format!("File '{}' not found.", filename));
  // }
  // };

  // let metadata = match file.metadata().await {
  // Ok(m) => m,
  // Err(e) => {
  // eprintln!("Failed to get file metadata for '{}': {}", filepath, e);
  // return HttpResponse::InternalServerError().body("Failed to get file metadata.");
  // }
  // };
  // let file_size = metadata.len();

  // let stream = FramedRead::new(file, BytesCodec::new());

  // HttpResponse::Ok()
  // .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", "None")))
  // .insert_header(("Content-Type", "application/octet-stream"))
  // .insert_header(("Content-Length", file_size.to_string()))
  // .streaming(stream)
}
