use actix_web::{HttpResponse, Responder, get, post, web};

use cosmox_backend_api::{
    Context,
    api::{
        self,
        resource::{
            ResourceAddRequest, ResourceAddTagRequest, ResourceDeleteRequest, ResourceError,
            ResourceQueryRequest,
        },
    },
    message,
};
use cosmox_macros::actix_web_error;

use crate::into_message;

actix_web_error! {
    ResourceError {
        NotFound() => {code: 404},
        UrlConflict() => {code: 409},
        LevelParameterConflict => {code: 400},
        InvalidLevelRange => {code: 400},
        ContentParseError() => {code: 400},
        TooLarge() => {code: 413},
        DeletionConflict() => {code: 409},
        ProcessingConflict() => {code: 409},
        InternalError() => {code: 500},
    }
}

#[get("/{rid}")]
pub async fn get(ctx: web::ReqData<Context<'_>>, rid: web::Path<u64>) -> impl Responder {
    into_message!(api::resource::get(&mut ctx.into_inner(), *rid).await)
}

#[post("/add")]
pub async fn add(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<ResourceAddRequest>,
) -> impl Responder {
    into_message!(api::resource::add(&mut ctx.into_inner(), payload.into_inner()).await)
}

#[post("/delete")]
pub async fn delete(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<ResourceDeleteRequest>,
) -> impl Responder {
    into_message!(api::resource::delete(&mut ctx.into_inner(), params.into_inner()).await)
}

#[get("/query")]
pub async fn query(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<ResourceQueryRequest>,
) -> impl Responder {
    into_message!(api::resource::query(&mut ctx.into_inner(), params.into_inner()).await)
}

#[get("/{rid}/metadata")]
pub async fn get_metadata() -> impl Responder {
    HttpResponse::NotImplemented().body("Not implemented {rid}/metadata api")
}

#[post("/{rid}/tag/add")]
pub async fn add_tag(
    ctx: web::ReqData<Context<'_>>,
    rid: web::Path<u64>,
    payload: web::Json<ResourceAddTagRequest>,
) -> impl Responder {
    into_message!(api::resource::add_tag(&mut ctx.into_inner(), *rid, payload.into_inner()).await)
}
