use std::fmt::Display;
use std::str::FromStr;
use std::{borrow::Cow, collections::HashMap};

use actix_web::{HttpResponse, Responder, delete, get, post, web};

use chrono::Utc;
use cosmox_macros::{ActixWebError, auto_webapi_doc, page_helper};
use sea_orm::{
  ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
  QueryFilter, QueryOrder, SqlErr,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::security::auth;
use crate::into_message;
use crate::utils::default_constants::default_page_size;
use crate::{entities::users, utils::message::Message};
use validator::{Validate, ValidationErrorsKind};

#[derive(Debug, Validate, Serialize, Deserialize, ToSchema)]
pub struct UserSignUpRequest {
  pub username: String,
  pub nickname: Option<String>,
  #[validate(length(
    min = 6,
    max = 128,
    message = "The 'password' field must be between 6 and 128 characters."
  ))]
  pub password: String,
  pub confirm_password: String,
  #[validate(email(message = "The 'email' field has an incorrect format."))]
  pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserLoginRequest {
  #[serde(flatten)]
  pub ident: UserIdent,
  pub password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum UserIdent {
  #[serde(rename = "username")]
  Username(String),
  #[serde(rename = "email")]
  Email(String),
}

impl Display for UserIdent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      UserIdent::Email(ident) => write!(f, "{}", ident),
      UserIdent::Username(ident) => write!(f, "{}", ident),
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserToken {
  pub uid: u64,
  pub username: String,
  pub token: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserResp {
  pub uid: u64,
  pub username: String,
  pub email: Option<String>,
}

#[page_helper]
#[derive(Debug, Deserialize, IntoParams)]
pub struct UserQueryRequest {
  pub status: Option<String>,
  pub role: Option<String>,
  pub search: Option<String>,
}

/// Errors related to user operations.
#[derive(Debug, thiserror::Error, ActixWebError)]
pub enum UserError {
  #[error("User not found with ID: {0}")]
  #[code(404)]
  NotFound(u64),

  #[error("User '{0}' is not authorized to perform this action.")]
  #[code(403)]
  Unauthorized(String),

  #[error("Username or email '{0}' is already taken")]
  #[code(409)]
  IdentTaken(String),

  #[error("Invalid password provided.")]
  #[code(401)]
  InvalidPassword,

  #[error("Validate failed")]
  #[code(409)]
  Validation(HashMap<Cow<'static, str>, ValidationErrorsKind>),

  #[error("User account '{0}' is locked.")]
  #[code(403)]
  AccountLocked(String),

  #[error("Email address '{0}' is already registered.")]
  #[code(409)]
  EmailAlreadyRegistered(String),

  #[error("Password confirm failed")]
  #[code(409)]
  ConfirmationPasswordMismatch,

  #[error("Failed to create user: {0}")]
  #[code(500)]
  UserCreationFailed(String),

  #[error("User {0} login failed")]
  #[code(403)]
  LoginFailed(String),

  /// Indicates an unexpected server-side issue.
  #[error("Internal server error: {0}")]
  #[code(500)]
  InternalError(String),
}

/// Sign up
///
/// Create a new user
#[auto_webapi_doc]
#[post("signUp")]
pub async fn sign_up(
  body: web::Json<UserSignUpRequest>,
  db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, UserError> {
  match body.validate() {
    Ok(_) => {
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
        Ok(user) => {
          let user_resp = UserResp {
            uid: user.uid,
            username: user.username,
            email: user.email,
          };
          Ok(HttpResponse::Ok().json(Message::ok(Some(user_resp))))
        }
        Err(err) => {
          if let Some(sqlerr) = err.sql_err()
            && let SqlErr::UniqueConstraintViolation(message) = sqlerr
            && message.contains("username")
          {
            Err(UserError::IdentTaken(body.username.clone()))
          } else {
            Err(UserError::InternalError("Unknown error".into())) // other database error
          }
        }
      }
    }
    Err(err) => Err(UserError::Validation(err.errors().clone())),
  }
}

/// Delete user
#[auto_webapi_doc]
#[delete("delete")]
pub async fn delete(uid: web::Query<u64>, db: web::Data<DatabaseConnection>) -> impl Responder {
  let user = users::ActiveModel {
    uid: Set(uid.0),
    ..Default::default()
  };
  match user.delete(db.as_ref()).await {
    Ok(_) => HttpResponse::Ok().body(""),
    Err(_) => HttpResponse::InternalServerError().body(""),
  }
}

/// Query User
#[auto_webapi_doc]
#[get("query")]
pub async fn query(
  params: web::Query<UserQueryRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
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
  HttpResponse::Ok().json(Message::ok(Some(result)).page(
    paginator.num_items().await.unwrap(),
    params.page_size,
    paginator.cur_page(),
    "",
  ))
}

/// get user
///
/// get user entity by uid
#[auto_webapi_doc]
#[get("{uid}")]
pub async fn get(
  uid: web::Path<u64>,
  db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, UserError> {
  let user = users::Entity::find_by_id(*uid)
    .one(db.as_ref())
    .await
    .unwrap();
  if let Some(mut user) = user {
    user.password = String::from("hidden");
    Ok(HttpResponse::Ok().json(Message::ok(Some(user))))
  } else {
    Err(UserError::NotFound(uid.into_inner()))
  }
}

/// Login
///
/// Login by username or email
#[auto_webapi_doc]
#[post("login")]
pub async fn login(
  body: web::Json<UserLoginRequest>,
  db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, UserError> {
  println!("{}", serde_json::to_string_pretty(&body).unwrap());
  let user = match &body.ident {
    UserIdent::Username(username) => users::Entity::find()
      .filter(users::Column::Username.eq(username))
      .all(db.as_ref())
      .await
      .unwrap(),
    UserIdent::Email(email) => users::Entity::find()
      .filter(users::Column::Email.eq(email))
      .all(db.as_ref())
      .await
      .unwrap(),
  };
  if let Some(user) = user.first() {
    match auth::verify_password(&body.password, &user.password) {
      Ok(_) => {
        // generate token
        let token = auth::generate_jwt(&user.uid.to_string(), auth::get_jwt_secret_key())
          .inspect_err(|err| log::error!("{err}"))
          .map_err(|_err| UserError::InternalError("Token generate error".to_string()));
        into_message!(token)
      }
      Err(err) => {
        if let argon2::password_hash::Error::Password = err {
          Err(UserError::InvalidPassword)
        } else {
          Err(UserError::LoginFailed(body.ident.to_string()))
        }
      }
    }
  } else {
    Err(UserError::NotFound(1))
  }
}

/// upload avatar
///
/// upload a small picture as your account's avatar
#[auto_webapi_doc]
#[post("{uid}/uploadAvatar")]
pub async fn upload_avatar(_uid: web::Path<u64>) -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented {uid}/uploadAvatar api")
}

#[auto_webapi_doc]
#[post("link/role/add")]
pub async fn role_add() -> impl Responder {
  HttpResponse::NotImplemented().body("Not implemented link/role/add api")
}
