use actix_web::{
    HttpResponse, Responder, get, post,
    web::{self, Payload},
};
use cosmox_backend_api::{
    Context,
    api::{
        self,
        plugin::{InstallPluginParams, PluginError, PluginName},
    },
    message,
};
use cosmox_macros::actix_web_error;

use cosmox_backend_api::api::plugin::PluginQueryRequest;
use serde_qs::web::QsQuery;

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
    payload: Payload,
) -> impl Responder {
    into_message!(
        api::plugin::install_plugin(&mut ctx.into_inner(), params.into_inner(), payload).await
    )
}

#[post("/uninstall")]
pub async fn uninstall_plugin(_ctx: web::ReqData<Context<'_>>) -> impl Responder {
    HttpResponse::NotImplemented().body("Uninstall plugin api is not yet implemented.")
}

#[post("/{plugin}/enable")]
pub async fn enable_plugin(
    ctx: web::ReqData<Context<'_>>,
    plugin: web::Path<String>,
) -> impl Responder {
    into_message!(
        api::plugin::enable_plugin(&mut ctx.into_inner(), PluginName::new(plugin.into_inner()))
            .await
    )
}

#[post("/{plugin}/disable")]
pub async fn disable_plugin(
    ctx: web::ReqData<Context<'_>>,
    plugin: web::Path<String>,
) -> impl Responder {
    into_message!(
        api::plugin::disable_plugin(&mut ctx.into_inner(), PluginName::new(plugin.into_inner()))
            .await
    )
}

#[get("/query")]
pub async fn query_plugins(
    ctx: web::ReqData<Context<'_>>,
    params: QsQuery<PluginQueryRequest>,
) -> impl Responder {
    into_message!(api::plugin::query(&mut ctx.into_inner(), params.into_inner()).await)
}

#[get("/info")]
pub async fn info(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(api::plugin::info(&mut ctx.into_inner()).await)
}
