use actix_web::{HttpResponse, Responder, get, post, web};
use cosmox_backend_api::{
    Context,
    api::{self, scanner::ScannerError},
    message,
};
use cosmox_macros::actix_web_error;

use crate::into_message;

actix_web_error! {
    ScannerError {
        NotFound() => {code: 404},
        Unauthorized => {code: 403},
        AlreadyRunning() => {code: 409},
        StartFailed() => {code: 500},
        TaskFailed() => {code: 500},
        InvalidConfiguration() => {code: 400},
        InvalidScanPath() => {code: 400},
        NoAvailableScanner => {code: 503},
        InternalError() => {code: 500},
    }
}

/// Scan library by id
#[post("/scan/{lid}")]
pub async fn scan(ctx: web::ReqData<Context<'_>>, lid: web::Path<u64>) -> impl Responder {
    into_message!(api::scanner::scan(&mut ctx.into_inner(), *lid).await)
}

/// Scan all libraries
#[post("/scan/all")]
pub async fn scan_all(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(api::scanner::scan_all(&mut ctx.into_inner()).await)
}

#[post("/task/add")]
pub async fn add_task(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    HttpResponse::NotImplemented().body("Add task api is not yet implemented.")
}

/// get status of scanner
///
#[get("/status")]
pub async fn get_status(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(api::scanner::processed(&mut ctx.into_inner()).await)
}

/// get information of scanner
///
#[get("/info")]
pub async fn info(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(api::scanner::info(&mut ctx.into_inner()).await)
}
