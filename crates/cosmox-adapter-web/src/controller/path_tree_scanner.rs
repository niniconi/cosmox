use actix_web::{Responder, get, web};
use cosmox_backend_api::{api::path_tree::PathTreeError, message};
use cosmox_macros::actix_web_error;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::into_message;

actix_web_error! {
    PathTreeError {
        IoError() => {code: 500},
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetSubPathParams {
    path: String,
    show_hide: Option<bool>,
}

#[get("/path/list")]
pub async fn get_sub_path(
    ctx: web::ReqData<cosmox_backend_api::Context<'_>>,
    params: web::Query<GetSubPathParams>,
) -> impl Responder {
    into_message!(
        cosmox_backend_api::api::path_tree::get_sub_path(
            &mut ctx.into_inner(),
            params.path.clone(),
            params.show_hide.unwrap_or(false),
        )
        .await
    )
}
