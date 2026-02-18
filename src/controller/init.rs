use actix_web::{HttpResponse, Responder, post, web};
use chrono::Utc;
use cosmox_macros::auto_webapi_doc;
use sea_orm::{
  ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr,
  sea_query::Query,
};
use serde::{Deserialize, Serialize};
use std::{fs::File, sync::atomic::Ordering};
use utoipa::ToSchema;

use crate::{
  configuration::Configuration,
  entities::{roles, users, users_related_roles},
  user::security::auth,
};

#[derive(Debug, Serialize)]
struct Status {
  initialized: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
struct InitializeConfig {
  admin_password: String,
  admin_confirm_password: String,
}

#[auto_webapi_doc]
#[post("initialize")]
pub async fn initialize(
  initialize_config: web::Json<InitializeConfig>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  let is_initialized =
    if initialize_config.admin_password == initialize_config.admin_confirm_password {
      let hash_password = auth::hash_password(&initialize_config.admin_password).unwrap();
      let current_navie_datetime = Utc::now().naive_utc();
      let user = users::ActiveModel {
        username: Set("admin".to_string()),
        password: Set(hash_password),
        last_update_datetime: Set(current_navie_datetime),
        create_datetime: Set(current_navie_datetime),
        ..Default::default()
      };

      let init_task = async || -> Result<bool, DbErr> {
        let user = user.insert(db.as_ref()).await?;
        let bind_roles = Query::insert()
          .into_table(users_related_roles::Entity)
          .columns([
            users_related_roles::Column::Uid,
            users_related_roles::Column::Rid,
          ])
          .values_panic([
            user.uid.into(),
            Query::select()
              .columns([roles::Column::Rid])
              .from(roles::Entity)
              .and_where(roles::Column::Name.eq("Administrator"))
              .take()
              .into(),
          ])
          .to_owned();
        db.execute(&bind_roles).await?;

        Ok(
          File::create(".first_boot.lock")
            .inspect_err(|err| log::error!("Create first_boot lock file error:{err}"))
            .is_ok(),
        )
      };

      match init_task().await {
        Ok(status) => status,
        Err(err) => {
          log::error!("{err}");
          false
        }
      }
    } else {
      false
    };

  if is_initialized {
    Configuration::get_global_configuration()
      .state
      .is_first_boot
      .store(false, Ordering::Relaxed);
  }

  HttpResponse::Ok().json(Status {
    initialized: is_initialized,
  })
}
