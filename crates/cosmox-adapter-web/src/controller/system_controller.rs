use crate::into_message;
use actix_web::{Responder, get, post, web};
use cosmox_backend_api::{
    Context,
    api::system::{self, SystemError},
    message,
};
use cosmox_macros::actix_web_error;

actix_web_error! {
    SystemError {
        NotFound() => {code: 404},
        Unauthorized() => {code: 403},
        AlreadyInState() => {code: 409},
        OperationFailed() => {code: 500},
        InvalidState() => {code: 400},
        ConfigurationError() => {code: 500},
        ShutdownInitiated => {code: 202}, // Specific for async operations that might not immediately fail
        InternalError() => {code: 500},
    }
}

#[get("/info")]
pub async fn info(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(system::info(&mut ctx.into_inner()).await)
}

/// Restart server
#[post("/restart")]
pub async fn restart(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(system::restart(&mut ctx.into_inner()).await)
}

#[post("/shutdown")]
pub async fn shutdown(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(system::shutdown(&mut ctx.into_inner()).await)
}

#[get("/about")]
pub async fn about(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(system::about(&mut ctx.into_inner()).await)
}

#[get("/log")]
pub async fn log(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(system::log(&mut ctx.into_inner()).await)
}

#[post("/all/delete")]
pub async fn delete_all(ctx: web::ReqData<Context<'_>>) -> impl Responder {
    into_message!(system::delete_all(&mut ctx.into_inner()).await)
}
