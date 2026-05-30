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
        let token = req
      .headers()
      .get("Authorization")
      .cloned()
      .map(|x| Token(x.to_str().map(|x| x.to_string()).inspect_err(|_|
          log::error!(
            "Failed to convert auth token `HeaderValue` to `String`: invalid utf-8 or non-ascii characters"
          )).ok()))
      .unwrap_or(Token(None));

        let srv = self.service.clone();
        Box::pin(async move {
            let ctx = Context::builder().token(token).build();
            req.extensions_mut().insert(ctx);
            let fut = srv.call(req);
            let res = fut.await?;
            Ok(res)
        })
    }
}
