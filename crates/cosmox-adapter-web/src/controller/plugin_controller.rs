use actix_web::{HttpResponse, Responder, get, post, web};
use cosmox_backend_api::{
    Context,
    api::{self, plugin::PluginError},
    message,
};
use cosmox_macros::{actix_web_error, page_helper};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::into_message;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InstallPluginParams {
    pub url: Option<String>,
}

#[page_helper]
#[derive(Debug, Deserialize, IntoParams)]
pub struct PluginQueryRequest {}

actix_web_error! {
    PluginError {
        NotFound() => {code: 404},
        Unauthorized => {code: 403},
        Disabled() => {code: 403},
        AlreadyInstalled() => {code: 409},
        LoadFailed() => {code: 500},
        WasmError() => {code: 500},
        IoError() => {code: 500},
        DownloadError() => {code: 504},
        InternalError() => {code: 500},
    }
}
#[post("/install")]
pub async fn install_plugin(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<InstallPluginParams>,
) -> impl Responder {
    HttpResponse::NotImplemented().body(format!(
        "Install plugin api is not yet implemented. payload = {payload:#?}"
    ))
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
