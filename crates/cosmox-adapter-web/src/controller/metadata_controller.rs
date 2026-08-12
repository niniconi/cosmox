use actix_web::{Responder, get, web};
use cosmox_backend_api::{
    Context,
    api::{
        self,
        metadata::{MetadataError, MetadataQueryKey},
    },
    message,
};
use cosmox_macros::actix_web_error;
use serde::Deserialize;

use crate::into_message;

actix_web_error! {
    MetadataError {
        NotFound() => {code: 404},
        InternalError() => {code: 500},
    }
}

#[derive(Deserialize)]
struct QueryParams {
    depth: usize,
}

#[get("/{rid}")]
pub async fn get(ctx: web::ReqData<Context<'_>>, rid: web::Path<u64>) -> impl Responder {
    into_message!(api::metadata::get(&mut ctx.into_inner(), rid.into_inner()).await)
}

/// Query metadata from server
#[get("/query/root")]
pub async fn query_root(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<QueryParams>,
) -> impl Responder {
    into_message!(
        api::metadata::query(&mut ctx.into_inner(), MetadataQueryKey::Root, params.depth).await
    )
}

/// Query metadata from server
#[get("/query/{rid}")]
pub async fn query_by_id(
    ctx: web::ReqData<Context<'_>>,
    rid: web::Path<u64>,
    params: web::Query<QueryParams>,
) -> impl Responder {
    into_message!(
        api::metadata::query(
            &mut ctx.into_inner(),
            MetadataQueryKey::Id(rid.into_inner()),
            params.depth
        )
        .await
    )
}
