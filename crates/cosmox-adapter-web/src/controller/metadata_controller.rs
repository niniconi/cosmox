use std::sync::Arc;

use actix_web::{Responder, get, web};
use cosmox_backend_api::{
    Context,
    api::{
        self,
        metadata::{MetadataError, MetadataQueryRequest},
    },
    message,
};
use cosmox_macros::actix_web_error;

use crate::into_message;

actix_web_error! {
    MetadataError {
        NotFound() => {code: 404},
        InternalError() => {code: 500},
    }
}

#[get("/{rid}")]
pub async fn get(ctx: web::ReqData<Context<'_>>, rid: web::Path<u64>) -> impl Responder {
    into_message!(api::metadata::get(&mut ctx.into_inner(), rid.into_inner()).await)
}

/// Query metadata from server
#[get("/query")]
pub async fn query(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<MetadataQueryRequest>,
) -> impl Responder {
    into_message!(api::metadata::query(&mut ctx.into_inner(), Arc::new(params.into_inner())).await)
}
