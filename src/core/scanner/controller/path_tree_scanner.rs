use std::fs;

use actix_web::{HttpResponse, Responder, get, web};
use cosmox_macros::auto_webapi_doc;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{into_message, utils};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetSubPathParams {
  path: String,
  show_hide: Option<bool>,
}

#[auto_webapi_doc]
#[get("/path/sub_path")]
pub async fn get_sub_path(params: web::Query<GetSubPathParams>) -> impl Responder {
  let path = &params.path;
  let show_hide = params.show_hide.unwrap_or(false);

  match fs::read_dir(path) {
    Ok(dir) => {
      let result: Vec<_> = dir
        .filter_map(|x| {
          if let Ok(entry) = x
            && let Ok(metadata) = entry.metadata()
            && metadata.is_dir()
            && let Some(dir_name) = entry.file_name().to_str()
          {
            if !show_hide
              && let Ok(is_hide) = utils::fs::is_hide(entry.path())
              && is_hide
            {
              return None;
            }
            Some(dir_name.to_string())
          } else {
            None
          }
        })
        .collect();
      into_message!(Ok::<_, ()>(result)).unwrap()
    }
    Err(err) => {
      // TODO impl error handle
      HttpResponse::Ok().body(format!("{err}"))
    }
  }
}
