use std::fs::File;

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, sea_query::Query,
};
use serde::{Deserialize, Serialize};

use crate::{
    entities::{roles, users, users_related_roles},
    get_db_connection,
    services::auth,
};

#[derive(Debug, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct InitializeConfig {
    pub admin_password: String,
    pub admin_confirm_password: String,
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub initialized: bool,
}

pub async fn initialize(initialize_config: InitializeConfig) -> bool {
    let db = get_db_connection().await;
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

            Ok(File::create(".first_boot.lock")
                .inspect_err(|err| log::error!("Create first_boot lock file error:{err}"))
                .is_ok())
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
    }
}
