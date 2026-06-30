use std::fs::File;

use actix_files::NamedFile;
use actix_multipart::{Field, Multipart, MultipartError};
use actix_web::{
    HttpRequest, HttpResponse, Responder, get, post,
    web::{self, Payload},
};
use bytes::Bytes;
use cosmox_backend_api::{
    Context,
    io::{
        self,
        transfer::{FileError, ItemFile},
    },
    message::{self, ApiError},
};
use cosmox_macros::actix_web_error;
use futures_util::{Stream, StreamExt};

use crate::{into_message, message::Wrapper};

actix_web_error! {
    FileError {
        NotFound() => {code: 404},
        Unauthorized() => {code: 403},
        AlreadyExists() => {code: 409},
        UploadFailed() => {code: 500},
        DownloadFailed() => {code: 500},
        TooLarge() => {code: 413},
        InvalidFileType() => {code: 400},
        NotSupportedScheme() => {code: 500},
        NotSupportedContentType() => {code: 500},
        InsufficientStorage => {code: 507},
        // Gateway Timeout if waiting for external storage, or Request Timeout if internal file processing took too long
        OperationTimeout() => {code: 504},
        PathTraversalAttack() => {code: 400},
        InternalError() => {code: 500},
    }
}

struct WrapperForMultipartField<T>(pub T);

impl Stream for WrapperForMultipartField<Field> {
    type Item = Result<Bytes, MultipartError>;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.0).poll_next(cx)
    }
}

impl ItemFile for WrapperForMultipartField<Field> {
    fn name(&mut self) -> Option<&str> {
        Field::name(&self.0)
    }
}

#[post("/push")]
pub async fn push(
    ctx: web::ReqData<Context<'_>>,
    request: HttpRequest,
    payload_octet_stream: Payload,
    payload_multipart: Multipart,
) -> impl Responder {
    let content_type = request
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");
    match content_type {
        t if t.contains("multipart/form-data") => {
            let payload_multipart = payload_multipart.map(|x| x.map(WrapperForMultipartField));
            into_message!(
                io::transfer::push_item_by_multipart_stream(
                    &mut ctx.into_inner(),
                    payload_multipart
                )
                .await
            )
        }
        "application/octet-stream" => {
            into_message!(
                io::transfer::push_item_by_octet_stream(
                    &mut ctx.into_inner(),
                    payload_octet_stream
                )
                .await
            )
        }
        "application/json" => Ok(HttpResponse::NotImplemented()
            .body("Not implemented upload file by `application/json`")),
        "application/x-www-form-urlencoded" => Ok(HttpResponse::NotImplemented()
            .body("Not implemented upload file by `application/x-www-form-urlencoded`")),
        _ => Err(Wrapper(ApiError::Logic(
            FileError::NotSupportedContentType(content_type.to_string()),
        ))),
    }
}

/// get item from server
#[get("/{id}/pull")]
pub async fn pull(
    ctx: web::ReqData<Context<'_>>,
    file_id: web::Path<u64>,
    _req: HttpRequest,
) -> impl Responder {
    io::transfer::pull_item_by_named_file(&mut ctx.into_inner(), file_id.into_inner())
        .await
        .and_then(|file| {
            let file = match File::open(file) {
                Ok(file) => file,
                Err(err) => {
                    return Err(ApiError::Logic(FileError::InternalError(err.to_string())));
                }
            };
            NamedFile::from_file(file, "ou")
                .map_err(|err| ApiError::Logic(FileError::InternalError(err.to_string())))
        })
        .map_err(Wrapper)
}
