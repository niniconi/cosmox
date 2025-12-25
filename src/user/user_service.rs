use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use sea_orm::{
  ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
  QueryFilter, QueryOrder, SqlErr,
};

use crate::{
  entities::users,
  user::{
    security::auth,
    user_controller::{
      UserError, UserIdent, UserLoginIdent, UserLoginRequest, UserQueryRequest, UserResp,
      UserSignUpRequest,
    },
  },
  utils::message::Pagination,
};

pub async fn get_user(uid: u64, db: Arc<DatabaseConnection>) -> Result<users::Model, UserError> {
  let user = users::Entity::find_by_id(uid)
    .one(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| UserError::InternalError("Database error".to_string()))?;

  user
    .ok_or(UserError::NotFound(UserIdent::Uid(uid)))
    .map(|mut user| {
      user.password = "hidden".to_string();
      user
    })
}

pub async fn sign_up(
  body: Arc<UserSignUpRequest>,
  db: Arc<DatabaseConnection>,
) -> Result<UserResp, UserError> {
  if body.confirm_password != body.password {
    return Err(UserError::ConfirmationPasswordMismatch);
  }
  log::debug!("sign up user {body:#?}");
  let hash_password = auth::hash_password(&body.password).unwrap();
  log::debug!("generate password hash {hash_password}");
  let current_navie_datetime = Utc::now().naive_utc();
  let user = users::ActiveModel {
    username: Set(body.username.to_owned()),
    password: Set(hash_password),
    nickname: Set(body.nickname.to_owned()),
    last_update_datetime: Set(current_navie_datetime),
    create_datetime: Set(current_navie_datetime),
    email: Set(body.email.to_owned()),
    ..Default::default()
  };

  match user.insert(db.as_ref()).await {
    Ok(user) => Ok(UserResp {
      uid: user.uid,
      username: user.username,
      email: user.email,
    }),
    Err(err) => {
      if let Some(sqlerr) = err.sql_err()
        && let SqlErr::UniqueConstraintViolation(message) = sqlerr
        && message.contains("username") // TODO check message.
      {
        Err(UserError::IdentTaken(body.username.clone()))
      } else {
        Err(UserError::InternalError("Unknown error".into())) // other database error
      }
    }
  }
}

/// Login
pub async fn login(
  payload: Arc<UserLoginRequest>,
  db: Arc<DatabaseConnection>,
) -> Result<String, UserError> {
  log::info!("user {} attempt login", payload.ident);
  let user = match &payload.ident {
    UserLoginIdent::Username(username) => users::Entity::find()
      .filter(users::Column::Username.eq(username))
      .all(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| UserError::InternalError("Database error".to_string()))?,
    UserLoginIdent::Email(email) => users::Entity::find()
      .filter(users::Column::Email.eq(email))
      .all(db.as_ref())
      .await
      .inspect_err(|err| log::error!("{err}"))
      .map_err(|_err| UserError::InternalError("Database error".to_string()))?,
  };

  if let Some(user) = user.first() {
    match auth::verify_password(&payload.password, &user.password) {
      Ok(_) => {
        // generate token
        auth::generate_jwt(&user.uid.to_string(), auth::get_jwt_secret_key())
          .inspect_err(|err| log::error!("{err}"))
          .map_err(|_err| UserError::InternalError("Token generate error".to_string()))
      }
      Err(err) => {
        if let argon2::password_hash::Error::Password = err {
          Err(UserError::InvalidUsernamePassword)
        } else {
          Err(UserError::LoginFailed(payload.ident.to_string()))
        }
      }
    }
  } else {
    Err(UserError::InvalidUsernamePassword)
  }
}

pub async fn delete(uid: u64, db: Arc<DatabaseConnection>) -> Result<(), UserError> {
  users::Entity::delete_by_id(uid)
    .exec(db.as_ref())
    .await
    .inspect_err(|err| log::error!("{err}"))
    .map(|_| ())
    .map_err(|_err| UserError::InternalError("Database error".to_string()))
}

pub async fn query(
  params: Arc<UserQueryRequest>,
  db: Arc<DatabaseConnection>,
) -> Result<(Vec<users::Model>, Pagination), UserError> {
  let mut select = users::Entity::find();
  let mut page = 0;

  if let Some(inner_page) = params.page {
    page = inner_page;
  }

  if let Some(search) = &params.search {
    select = select.filter(users::Column::Username.contains(search));
  };

  if let Some(sort) = &params.sort
    && let Ok(column) = users::Column::from_str(sort)
  {
    select = select.order_by(column, sea_orm::Order::Asc);
  };

  let paginator = select.paginate(db.as_ref(), params.page_size);
  let result = paginator.fetch_page(page).await.unwrap();
  let pagination = Pagination::new(
    paginator.num_items().await.unwrap(),
    params.page_size,
    paginator.cur_page(),
    "",
  );
  Ok((result, pagination))
}
