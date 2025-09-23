use chrono::{DateTime, Utc};
// use futures::future::LocalBoxFuture;
// use futures::future::{ready, LocalBoxFuture, Ready, FutureExt};
use actix_web::{
  Error, HttpResponse, Result,
  body::MessageBody,
  dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
  http::header::ContentType,
};
use futures_util::future::FutureExt; // For .boxed_local()
use futures_util::future::LocalBoxFuture;
use serde::{Deserialize, Serialize};
use std::future::{Ready, ready};
use std::rc::Rc;
use utoipa::ToSchema; // For sharing state in middleware

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Message<T> {
  pub code: String,
  pub message: String,
  pub status: String,
  pub datetime: DateTime<Utc>,
  #[serde(flatten)]
  pub payload: Option<MessagePayload<T>>,

  pub pagination: Option<Pagination>,
}

impl<T> Message<T> {
  pub fn ok(data: Option<T>) -> Message<T> {
    match data {
      Some(data) => Message {
        code: "Ok".to_string(),
        message: "".to_string(),
        status: "".to_string(),
        datetime: Utc::now(),
        payload: Some(MessagePayload::Data(data)),
        pagination: None,
      },
      None => Message {
        code: "Ok".to_string(),
        message: "".to_string(),
        status: "".to_string(),
        datetime: Utc::now(),
        payload: None,
        pagination: None,
      },
    }
  }

  pub fn page(
    &mut self,
    total_items: u64,
    page_size: u64,
    current_page: u64,
    _url_format: &'static str,
  ) -> &Self {
    self.pagination = Some(Pagination {
      total_items: total_items,
      total_pages: (total_items / page_size) + (total_items % page_size > 0) as u64,
      current_page: current_page,
      page_size: page_size,
      next_page_url: "".to_string(),
      prev_page_url: "".to_string(),
    });
    self
  }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum MessagePayload<T> {
  #[serde(rename = "errors")]
  Error(Vec<T>),
  #[serde(rename = "data")]
  Data(T),
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Pagination {
  pub total_items: u64,
  pub total_pages: u64,
  pub current_page: u64,
  pub page_size: u64,
  pub next_page_url: String,
  pub prev_page_url: String,
}
