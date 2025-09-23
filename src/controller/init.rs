use actix_web::{HttpResponse, Responder, post, web};
use cosmox_macros::auto_webapi_doc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{entities::users, user};

#[derive(Debug, Serialize)]
struct Status {
  initialized: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
struct InitializeConfig {
  admin_password: String,
  admin_confirm_passwod: String,
}

#[auto_webapi_doc]
#[post("initialize")]
pub async fn initialize(
  initialize_config: web::Json<InitializeConfig>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  let is_initialized = false;

  if is_initialized && initialize_config.admin_password == initialize_config.admin_confirm_passwod {
    let hash_password =
      user::security::user::hash_password(&initialize_config.admin_password).unwrap();
    let user = users::ActiveModel {
      username: Set("admin".to_string()),
      password: Set(hash_password),
      ..Default::default()
    };
    match user.insert(db.as_ref()).await {
      Ok(_user) => {
        todo!()
      }
      Err(_) => {
        todo!()
      }
    }
  }

  HttpResponse::Ok().json(Status {
    initialized: is_initialized,
  })
}
