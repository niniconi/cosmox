use actix_web::{Responder, post, web};
use cosmox_backend_api::{
    Context,
    api::{self, init::InitializeConfig},
};

use crate::into_message;

#[post("/initialize")]
pub async fn initialize(
    ctx: web::ReqData<Context<'_>>,
    payload: web::Json<InitializeConfig>,
) -> impl Responder {
    into_message!(api::init::initialize(&mut ctx.into_inner(), payload.into_inner()).await)
}
