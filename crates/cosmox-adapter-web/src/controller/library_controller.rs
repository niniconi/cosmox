use std::sync::Arc;

use actix_web::{Responder, get, post, web};
use cosmox_backend_api::{
    Context,
    api::{
        self,
        library::{LibraryAddRequest, LibraryError, LibraryQueryRequest, ModifyLibraryRequest},
    },
    message,
};
use cosmox_macros::actix_web_error;
use serde::Deserialize;

use crate::into_message;

#[derive(Deserialize)]
pub struct LibraryDeleteRequest {
    pub lid: u64,
}

actix_web_error! {
    LibraryError {
        NotFound() => {code: 404},
        Unauthorized() => {code: 403},
        NameConflict() => {code: 409},
        EmptyLibrary() => {code: 400},
        MetadataUpdateFailed() => {code: 500},
        DeletionConflict() => {code: 409},
        MaxLibrariesExceeded() => {code: 400},
        InternalError() => {code: 500},
    }
}

#[get("/{lid}")]
pub async fn get(ctx: web::ReqData<Context<'_>>, lid: web::Path<u64>) -> impl Responder {
    into_message!(api::library::get(&mut ctx.into_inner(), lid.into_inner()).await)
}

#[get("/query")]
pub async fn query(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<LibraryQueryRequest>,
) -> impl Responder {
    into_message!(api::library::query(&mut ctx.into_inner(), params.into_inner()).await)
}

#[post("/{lid}/modify")]
pub async fn modify(
    ctx: web::ReqData<Context<'_>>,
    lid: web::Path<u64>,
    payload: web::Json<ModifyLibraryRequest>,
) -> impl Responder {
    into_message!(
        api::library::modify(
            &mut ctx.into_inner(),
            lid.into_inner(),
            payload.into_inner()
        )
        .await
    )
}

/// add library
///
/// add library with tags and path in disk.
#[post("/add")]
pub async fn add(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<LibraryAddRequest>,
) -> impl Responder {
    into_message!(api::library::add(&mut ctx.into_inner(), Arc::new(payload.into_inner())).await)
}

/// delete library
///
/// delete the entity in database table library.
/// delete the metadata information in disk (Option)
#[post("/delete")]
pub async fn delete(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<LibraryDeleteRequest>,
) -> impl Responder {
    into_message!(api::library::delete(&mut ctx.into_inner(), params.lid).await)
}

/// Returns all selectable Types
#[get("/types/all")]
pub async fn get_all_types(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(api::library::get_all_type(&mut ctx.into_inner()).await)
}
