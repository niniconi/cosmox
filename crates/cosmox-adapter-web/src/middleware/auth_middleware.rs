use std::{
    future::{Ready, ready},
    rc::Rc,
};

use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use cosmox_backend_api::{Context, Token};
use futures_util::future::LocalBoxFuture;

const NEW_TOKEN_HEADER: &str = "x-new-token";

pub struct TokenExtractor;

impl<S, B> Transform<S, ServiceRequest> for TokenExtractor
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TokenAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TokenAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct TokenAuthMiddleware<S> {
    service: Rc<S>,
}

/// Extract the bearer token from the `Authorization` header.
///
/// Accepts both `Bearer <token>` and a bare token for compatibility with
/// clients that already send the raw JWT.
fn extract_token(header_value: Option<&str>) -> Token {
    let Some(value) = header_value else {
        return Token(None);
    };
    let stripped = value.strip_prefix("Bearer ").unwrap_or(value);
    Token(Some(stripped.to_string()))
}

impl<S, B> Service<ServiceRequest> for TokenAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|x| {
                x.to_str()
                    .inspect_err(|_| {
                        log::error!(
                            "Failed to convert auth token `HeaderValue` to `String`: invalid utf-8 or non-ascii characters"
                        )
                    })
                    .ok()
            });
        let token = extract_token(auth_header);

        let srv = self.service.clone();
        Box::pin(async move {
            let ctx = Context::builder().token(token).build();
            req.extensions_mut().insert(ctx);

            let mut res = srv.call(req).await?;

            // Sliding renewal: if access checks minted a fresh token, expose
            // it to the client so the next request can use it.
            let new_token = res
                .request()
                .extensions()
                .get::<Context>()
                .and_then(|ctx| ctx.access_ctx.new_token.lock().unwrap().as_ref().cloned());
            if let Some(new_token) = new_token {
                res.headers_mut().insert(
                    actix_web::http::header::HeaderName::from_static(NEW_TOKEN_HEADER),
                    new_token.parse().unwrap(),
                );
            }

            Ok(res)
        })
    }
}
