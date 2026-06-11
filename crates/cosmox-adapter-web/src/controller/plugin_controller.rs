use actix_web::{HttpResponse, Responder, get, post, web::{self, Payload}};
use cosmox_backend_api::{
    Context,
    api::{
        self,
        plugin::{InstallPluginParams, PluginError},
    },
    message,
};
use cosmox_macros::actix_web_error;

use crate::into_message;

actix_web_error! {
    PluginError {
        NotFound() => {code: 404},
        Unauthorized => {code: 403},
        Disabled() => {code: 403},
        AlreadyInstalled() => {code: 409},
        LoadFailed() => {code: 500},
        WasmError() => {code: 500},
        IoError() => {code: 500},
        HttpTransportError{} => {code: 502},
        NetworkTransportError{} => {code: 504},
        StreamTransportError{} => {code: 504},
        FileSystemError() => {code: 500},
        PathTraversalAttack() => {code: 400},
        InvalidPluginPackage() => {code: 422},
        InternalError() => {code: 500},
    }
}

#[post("/install")]
pub async fn install_plugin(
    ctx: web::ReqData<Context<'_>>,
    params: web::Query<InstallPluginParams>,
    payload: Payload
) -> impl Responder {
    into_message!(api::plugin::install_plugin(
        &mut ctx.into_inner(),
        params.into_inner(),
        payload
    ).await)
}

#[post("/uninstall")]
pub async fn uninstall_plugin(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    HttpResponse::NotImplemented().body("Uninstall plugin api is not yet implemented.")
}

#[post("/{id}/enable")]
pub async fn enable_plugin(ctx: web::ReqData<Context<'_>>, id: web::Path<u64>) -> impl Responder {
    HttpResponse::NotImplemented().body("Enable plugin api is not yet implemented.")
}

#[post("/{id}/disable")]
pub async fn disable_plugin(ctx: web::ReqData<Context<'_>>, id: web::Path<u64>) -> impl Responder {
    HttpResponse::NotImplemented().body("Disable plugin api is not yet implemented.")
}

#[get("/info")]
pub async fn info(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(api::plugin::info(&mut ctx.into_inner()).await)
}
