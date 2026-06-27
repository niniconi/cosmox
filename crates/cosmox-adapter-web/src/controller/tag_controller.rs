use actix_web::{Responder, get, post, web};
use cosmox_backend_api::{
    Context,
    api::{
        self,
        tag::{
            TagAddRequest, TagError, TagGroupAddRequest, TagGroupDeleteRequest,
            TagGroupQueryRequest, TagQueryRequest,
        },
    },
    message,
};
use cosmox_macros::actix_web_error;

use crate::into_message;

actix_web_error! {
    TagError {
        NotFound() => {code: 404},
        Unauthorized => {code: 403},
        AlreadyExists() => {code: 409},
        InvalidName() => {code: 400},
        MaxTagsExceeded() => {code: 400},
        ProtectedTag() => {code: 403},
        InternalError() => {code: 500},
    }
}

#[get("/{id}")]
pub async fn get(ctx: web::ReqData<Context<'_>>, tid: web::Path<u64>) -> impl Responder {
    into_message!(api::tag::get(&mut ctx.into_inner(), *tid).await)
}

#[get("/group/{id}")]
pub async fn get_group(ctx: web::ReqData<Context<'_>>, tgid: web::Path<u64>) -> impl Responder {
    into_message!(api::tag::get_group(&mut ctx.into_inner(), *tgid).await)
}

#[post("/add")]
pub async fn add(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<TagAddRequest>,
) -> impl Responder {
    into_message!(api::tag::add(&mut ctx.into_inner(), payload.into_inner()).await)
}

#[post("/group/add")]
pub async fn add_group(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<TagGroupAddRequest>,
) -> impl Responder {
    into_message!(api::tag::add_group(&mut ctx.into_inner(), payload.into_inner()).await)
}

#[post("/group/delete")]
pub async fn delete_group(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<TagGroupDeleteRequest>,
) -> impl Responder {
    into_message!(api::tag::delete_group(&mut ctx.into_inner(), params.into_inner()).await)
}

#[get("/query")]
pub async fn query(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<TagQueryRequest>,
) -> impl Responder {
    into_message!(api::tag::query(&mut ctx.into_inner(), params.into_inner()).await)
}

#[get("/group/query")]
pub async fn query_group(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<TagGroupQueryRequest>,
) -> impl Responder {
    into_message!(api::tag::query_group(&mut ctx.into_inner(), params.into_inner()).await)
}

#[get("/catalog")]
pub async fn catalog(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(api::tag::catalog(&mut ctx.into_inner()).await)
}
