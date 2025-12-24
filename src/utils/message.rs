use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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

impl Pagination {
  pub fn new(
    total_items: u64,
    page_size: u64,
    current_page: u64,
    _url_format: &'static str,
  ) -> Self {
    Pagination {
      total_items: total_items,
      total_pages: (total_items / page_size) + (!total_items.is_multiple_of(page_size)) as u64,
      current_page: current_page,
      page_size: page_size,
      next_page_url: "".to_string(),
      prev_page_url: "".to_string(),
    }
  }
}

#[macro_export]
macro_rules! into_message {
  ($result:expr $(,$ident:ident = $value:expr)*) => {
    match $result {
      Ok(data) => {
        Ok(actix_web::HttpResponse::Ok().json($crate::utils::message::Message {
          $($ident: $value,)*
          payload: Some($crate::utils::message::MessagePayload::Data(data)),
          ..Default::default()
        }))
      },
      Err(err) => Err(err)
    }
  };
}

#[macro_export]
macro_rules! into_message_page {
  ($result:expr) => {
    match $result {
      Ok((data, pagination)) => {
        let message = $crate::utils::message::Message {
          pagination: Some(pagination),
          payload: Some($crate::utils::message::MessagePayload::Data(data)),
          ..Default::default()
        };
        Ok(actix_web::HttpResponse::Ok().json(message))
      }
      Err(err) => Err(err),
    }
  };
}
