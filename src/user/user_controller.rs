use std::fmt::{Debug, Display};
use std::sync::Arc;
use std::{borrow::Cow, collections::HashMap};

use actix_web::{HttpResponse, Responder, delete, get, post, web};

use cosmox_macros::{ActixWebError, auto_webapi_doc, page_helper};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::user::user_service;
use crate::{into_message, into_message_page};
use validator::{Validate, ValidationError, ValidationErrorsKind};

#[derive(Debug, Validate, Serialize, Deserialize, ToSchema)]
pub struct UserSignUpRequest {
  #[validate(
    length(
      min = 1,
      max = 128,
      message = "The `username` field must be between 1 and 128 characters."
    ),
    custom(function = "validate_username")
  )]
  pub username: String,

  #[validate(length(
    min = 1,
    max = 128,
    message = "The `nickname` field must be between 1 and 128 characters."
  ))]
  pub nickname: Option<String>,
  #[validate(length(
    min = 6,
    max = 128,
    message = "The `password` field must be between 6 and 128 characters."
  ))]
  pub password: String,
  #[validate(length(
    min = 6,
    max = 128,
    message = "The `password` field must be between 6 and 128 characters."
  ))]
  pub confirm_password: String,
  #[validate(email(message = "The `email` field has an incorrect format."))]
  pub email: Option<String>,
}

#[derive(Debug, Validate, Serialize, Deserialize, ToSchema)]
pub struct UserLoginRequest {
  #[serde(flatten)]
  #[validate(custom(function = "validate_userident"))]
  pub ident: UserLoginIdent,
  #[validate(length(
    min = 6,
    max = 128,
    message = "The `password` field must be between 6 and 128 characters."
  ))]
  pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum UserLoginIdent {
  #[serde(rename = "username")]
  Username(String),
  #[serde(rename = "email")]
  Email(String),
}

pub enum UserIdent {
  Username(String),
  Email(String),
  Uid(u64),
}

impl From<UserLoginIdent> for UserIdent {
  fn from(value: UserLoginIdent) -> Self {
    match value {
      UserLoginIdent::Email(email) => UserIdent::Email(email.clone()),
      UserLoginIdent::Username(username) => UserIdent::Username(username.clone()),
    }
  }
}

impl Display for UserIdent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      UserIdent::Uid(uid) => write!(f, "uid: {}", uid),
      UserIdent::Username(username) => write!(f, "username: {}", username),
      UserIdent::Email(email) => write!(f, "email: {}", email),
    }
  }
}

impl Debug for UserIdent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(self, f)
  }
}

impl Display for UserLoginIdent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      UserLoginIdent::Email(ident) => write!(f, "{}", ident),
      UserLoginIdent::Username(ident) => write!(f, "{}", ident),
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
  #[error("User not found with {0}")]
  #[code(404)]
  NotFound(UserIdent),

  #[error("User '{0}' is not authorized to perform this action.")]
  #[code(403)]
  Unauthorized(String),

  #[error("Username or email '{0}' is already taken")]
  #[code(409)]
  IdentTaken(String),

  #[error("Invalid password or username provided.")]
  #[code(401)]
  InvalidUsernamePassword,

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

pub fn validate_username(username: &str) -> Result<(), ValidationError> {
  let mut username = username.chars();
  let mut result = Ok(());
  while let Some(ch) = username.next() {
    if !matches!(ch,'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' ) {
      result = Err(ValidationError::new(
        "The `username` field must consist only of underscores, hyphens, letters and digits.",
      ));
      break;
    }
  }
  result
}

pub fn validate_userident(ident: &UserLoginIdent) -> Result<(), ValidationError> {
  match ident {
    UserLoginIdent::Email(email) => {
      if validator::ValidateEmail::validate_email(email) {
        Ok(())
      } else {
        Err(ValidationError::new(
          "The `email` field has an incorrect format.",
        ))
      }
    }
    UserLoginIdent::Username(username) => validate_username(username),
  }
}

/// Sign up
///
/// Create a new user
#[auto_webapi_doc]
#[post("signUp")]
pub async fn sign_up(
  payload: web::Json<UserSignUpRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  if let Err(err) = payload.validate() {
    Err(UserError::Validation(err.errors().clone()))
  } else {
    into_message!(user_service::sign_up(Arc::new(payload.into_inner()), db.into_inner()).await)
  }
}

/// Delete user
#[auto_webapi_doc]
#[delete("delete")]
pub async fn delete(uid: web::Query<u64>, db: web::Data<DatabaseConnection>) -> impl Responder {
  into_message!(user_service::delete(*uid, db.into_inner()).await)
}

/// Query User
#[auto_webapi_doc]
#[get("query")]
pub async fn query(
  params: web::Query<UserQueryRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  into_message_page!(user_service::query(Arc::new(params.into_inner()), db.into_inner()).await)
}

/// get user
///
/// get user entity by uid
#[auto_webapi_doc]
#[get("{uid}")]
pub async fn get(uid: web::Path<u64>, db: web::Data<DatabaseConnection>) -> impl Responder {
  into_message!(user_service::get_user(*uid, db.into_inner()).await)
}

/// Login
///
/// Login by username or email
#[auto_webapi_doc]
#[post("login")]
pub async fn login(
  payload: web::Json<UserLoginRequest>,
  db: web::Data<DatabaseConnection>,
) -> impl Responder {
  if let Err(err) = payload.validate() {
    Err(UserError::Validation(err.errors().clone()))
  } else {
    into_message!(user_service::login(Arc::new(payload.into_inner()), db.into_inner()).await)
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
