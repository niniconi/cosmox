use actix_web::{Responder, get, web};
use cosmox_backend_api::{
    Context,
    api::{
        self,
        search::{SearchError, SearchRequest},
    },
    message,
};
use cosmox_macros::actix_web_error;
use serde_qs::web::QsQuery;

use crate::into_message;

actix_web_error! {
    SearchError {
        Unauthorized => {code: 403},
        InternalError() => {code: 403},
    }
}

#[get("/search")]
pub async fn search(
    ctx: web::ReqData<Context<'_>>,
    params: QsQuery<SearchRequest>,
) -> impl Responder {
    into_message!(api::search::search(&mut ctx.into_inner(), params.into_inner()).await)
}
