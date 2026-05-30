use core::{default::Default, future::Future, marker::Sized, result::Result};
use std::fmt::Display;

use chrono::{DateTime, Utc};
use common::message::Pagination;
use serde::{Deserialize, Serialize};

use crate::{Context, auth::access_check};

pub use cosmox_backend_data::services::role_permission_service::AuthError;

#[derive(Debug)]
pub enum ApiError<E> {
    Logic(E),
    Auth(AuthError),
}

impl<T: Display> Display for ApiError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Auth(err) => write!(f, "{}", err),
            ApiError::Logic(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message<T> {
    pub code: String,
    pub message: String,
    pub status: String,
    pub datetime: DateTime<Utc>,
    #[serde(flatten)]
    pub payload: Option<MessagePayload<T>>,

    pub pagination: Option<Pagination>,
}

impl<T> Default for Message<T> {
    fn default() -> Self {
        Self {
            code: "200".to_string(),
            message: "".to_string(),
            status: "success".to_string(),
            datetime: Utc::now(),
            payload: None,
            pagination: None,
        }
    }
}

pub trait FromService<T, E>
where
    Self: Sized,
{
    /// Execute a service future after authenticating the request.
    fn from_service(
        ctx: &mut Context<'_>,
        other: impl Future<Output = Result<T, E>>,
    ) -> impl Future<Output = Result<Self, ApiError<E>>>;

    /// Authenticate the request and execute a service closure with mutable access to [`Context`].
    ///
    /// Unlike [`from_service`](FromService::from_service), which accepts a pre-built future,
    /// this method passes the resolved `&mut Context<'_>` into the service closure,
    /// allowing access to authenticated user data such as `ctx.request_user.uid`.
    fn from_service_with_ctx<'ctx, F, Fut>(
        ctx: &'ctx mut Context<'ctx>,
        service: F,
    ) -> impl Future<Output = Result<Self, ApiError<E>>>
    where
        F: FnOnce(&'ctx mut Context<'ctx>) -> Fut,
        Fut: Future<Output = Result<T, E>> + 'ctx;
}

impl<T> From<T> for Message<T> {
    fn from(val: T) -> Self {
        Self {
            payload: Some(MessagePayload::Data(val)),
            ..Default::default()
        }
    }
}

impl<T, E> FromService<(T, Pagination), E> for Message<T> {
    async fn from_service(
        ctx: &mut Context<'_>,
        fut: impl Future<Output = Result<(T, Pagination), E>>,
    ) -> Result<Message<T>, ApiError<E>> {
        let auth_status = access_check::check_resource_access(ctx).await;

        match auth_status {
            Ok(req_user) => {
                ctx.request_user = req_user;
                match fut.await {
                    Ok((data, page)) => Ok(Message {
                        payload: Some(MessagePayload::Data(data)),
                        pagination: Some(page),
                        ..Default::default()
                    }),
                    Err(err) => Err(ApiError::Logic(err)),
                }
            }
            Err(err) => Err(ApiError::Auth(err)),
        }
    }

    async fn from_service_with_ctx<'ctx, F, Fut>(
        ctx: &'ctx mut Context<'ctx>,
        service: F,
    ) -> Result<Message<T>, ApiError<E>>
    where
        F: FnOnce(&'ctx mut Context<'ctx>) -> Fut,
        Fut: Future<Output = Result<(T, Pagination), E>> + 'ctx,
    {
        let auth_status = access_check::check_resource_access(ctx).await;
        match auth_status {
            Ok(req_user) => {
                ctx.request_user = req_user;
                match service(ctx).await {
                    Ok((data, page)) => Ok(Message {
                        payload: Some(MessagePayload::Data(data)),
                        pagination: Some(page),
                        ..Default::default()
                    }),
                    Err(err) => Err(ApiError::Logic(err)),
                }
            }
            Err(err) => Err(ApiError::Auth(err)),
        }
    }
}

impl<T, E> FromService<T, E> for Message<T> {
    async fn from_service(
        ctx: &mut Context<'_>,
        fut: impl Future<Output = Result<T, E>>,
    ) -> Result<Message<T>, ApiError<E>> {
        let auth_status = access_check::check_resource_access(ctx).await;

        match auth_status {
            Ok(req_user) => {
                ctx.request_user = req_user;
                match fut.await {
                    Ok(data) => Ok(Message::from(data)),
                    Err(err) => Err(ApiError::Logic(err)),
                }
            }
            Err(err) => Err(ApiError::Auth(err)),
        }
    }

    async fn from_service_with_ctx<'ctx, F, Fut>(
        ctx: &'ctx mut Context<'ctx>,
        service: F,
    ) -> Result<Message<T>, ApiError<E>>
    where
        F: FnOnce(&'ctx mut Context<'ctx>) -> Fut,
        Fut: Future<Output = Result<T, E>> + 'ctx,
    {
        let auth_status = access_check::check_resource_access(ctx).await;
        match auth_status {
            Ok(req_user) => {
                ctx.request_user = req_user;
                match service(ctx).await {
                    Ok(data) => Ok(Message::from(data)),
                    Err(err) => Err(ApiError::Logic(err)),
                }
            }
            Err(err) => Err(ApiError::Auth(err)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessagePayload<T> {
    #[serde(rename = "errors")]
    Error(Vec<T>),
    #[serde(rename = "data")]
    Data(T),
}
