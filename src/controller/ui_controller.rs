//! return dynamic gui's struct
//!
//!

use actix_web::{HttpResponse, Responder, get};
use cosmox_macros::auto_webapi_doc;

#[auto_webapi_doc]
#[get("/wasm")]
pub async fn get_core() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented wasm api")
}
