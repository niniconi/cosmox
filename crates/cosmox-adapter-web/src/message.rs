use std::{
    convert::Infallible,
    fmt::{Debug, Display},
};

use actix_web::{HttpResponse, ResponseError, body::BoxBody, http::StatusCode};
use chrono::{DateTime, Utc};
use common::message::Pagination;
use cosmox_backend_api::message::{self, ApiError, AuthError};

#[derive(Debug)]
pub(crate) struct Wrapper<T>(pub T);

impl<T> From<ApiError<T>> for Wrapper<ApiError<T>> {
    fn from(value: ApiError<T>) -> Self {
        Self(value)
    }
}

impl<T> Display for Wrapper<ApiError<T>>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
pub trait InnerResponseError {
    fn status_code(&self) -> StatusCode;
    fn error_response(
        &self,
        status: String,
        datetime: DateTime<Utc>,
        pagination: Option<Pagination>,
    ) -> HttpResponse<BoxBody>;
}

impl InnerResponseError for Infallible {
    fn status_code(&self) -> StatusCode {
        unreachable!()
    }
    fn error_response(
        &self,
        _status: String,
        _datetime: DateTime<Utc>,
        _pagination: Option<Pagination>,
    ) -> HttpResponse<BoxBody> {
        unreachable!()
    }
}

impl<T> ResponseError for Wrapper<ApiError<T>>
where
    T: InnerResponseError + Display + Debug,
{
    fn status_code(&self) -> StatusCode {
        match &self.0 {
            ApiError::Logic(err) => err.status_code(),
            ApiError::Auth(err) => match err {
                AuthError::Unauthorized(..) => actix_web::http::StatusCode::from_u16(401).unwrap(),
                AuthError::TokenExpired(..) => actix_web::http::StatusCode::from_u16(401).unwrap(),
                AuthError::Forbidden => actix_web::http::StatusCode::from_u16(403).unwrap(),
                AuthError::MissingData => actix_web::http::StatusCode::from_u16(400).unwrap(),
                AuthError::InvalidRole => actix_web::http::StatusCode::from_u16(403).unwrap(),
                AuthError::InternalError(..) => actix_web::http::StatusCode::from_u16(500).unwrap(),
            },
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let status = "failed".to_string();
        let datetime = chrono::Utc::now();
        let pagination = None;
        let code = self.status_code().to_string();
        let message = self.to_string();
        let payload = Option::<message::MessagePayload<u8>>::None;
        match &self.0 {
            ApiError::Logic(err) => err.error_response(status, datetime, pagination),
            ApiError::Auth(_) => {
                actix_web::HttpResponse::build(self.status_code()).json(message::Message {
                    code,
                    message,
                    status,
                    datetime,
                    payload,
                    pagination,
                })
            }
        }
    }
}

#[macro_export]
macro_rules! into_message {
    ($result:expr) => {{
        use $crate::message::Wrapper;
        match $result {
            Ok(message) => Ok(actix_web::HttpResponse::Ok().json(message)),
            Err(err) => Err(<Wrapper<cosmox_backend_api::message::ApiError<_>>>::from(
                err,
            )),
        }
    }};
}
