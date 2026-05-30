//! return dynamic gui's struct
//!
//!

use actix_web::{HttpResponse, Responder, get};

#[get("/wasm")]
pub async fn get_core() -> impl Responder {
    HttpResponse::NotImplemented().body("Not implemented wasm api")
}
