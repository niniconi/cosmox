use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use cosmox_backend_data::RequestUser;

use crate::api::Endpoint;

pub mod api;
pub mod auth;
pub mod io;
pub mod message;

pub type ContextHook = fn() -> i32;

#[derive(Debug, Clone, Default)]
pub struct Token(pub Option<String>);

impl Token {
    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

#[derive(Debug, Default, Clone)]
pub struct AccessContext {
    pub endpoint: Endpoint,
    pub token: Token,
    /// Fresh token emitted by sliding renewal; the web adapter attaches it
    /// to the response as the `X-New-Token` header.
    ///
    /// Shared slot: handlers receive a `Clone` of the `Context` inserted
    /// into the request extensions, and `Arc` keeps both copies pointing at
    /// the same cell, so the middleware can read back what the access
    /// check minted on its own copy. `Mutex` keeps the context `Send` for
    /// adapters that run API calls inside `tokio::spawn`.
    pub new_token: Arc<Mutex<Option<String>>>,
}

#[derive(Debug, Default, Clone)]
pub struct Context<'ctx> {
    pub access_ctx: AccessContext,
    pub request_user: RequestUser,
    pub lifetime: PhantomData<&'ctx u8>,
}

#[derive(Debug, Default)]
pub struct ContextBuilder<'ctx> {
    pub access_ctx: AccessContext,
    pub request_user: RequestUser,
    pub lifetime: PhantomData<&'ctx u8>,
}

impl<'ctx> Context<'ctx> {
    pub fn builder() -> ContextBuilder<'ctx> {
        ContextBuilder::default()
    }
}

impl<'ctx> ContextBuilder<'ctx> {
    #[inline]
    pub fn endpoint(mut self, endpoint: Endpoint) -> Self {
        self.access_ctx.endpoint = endpoint;
        self
    }

    #[inline]
    pub fn token(mut self, token: Token) -> Self {
        self.access_ctx.token = token;
        self
    }

    #[inline]
    pub fn request_user(mut self, request_user: RequestUser) -> Self {
        self.request_user = request_user;
        self
    }

    #[inline]
    pub fn build(self) -> Context<'ctx> {
        Context {
            access_ctx: self.access_ctx,
            request_user: self.request_user,
            lifetime: self.lifetime,
        }
    }
}
