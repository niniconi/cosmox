use core::fmt::Display;
use std::{borrow::Cow, collections::HashMap, io, path::PathBuf, str::FromStr, sync::Arc};

use bytes::Bytes;
use chrono::Utc;
use common::message::Pagination;
use cosmox_configuration::Configuration;
use cosmox_macros::page_helper;
use futures_util::StreamExt;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, SqlErr, Value, sea_query::expr::Expr,
};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError, ValidationErrorsKind};

use crate::{
    entities::users,
    get_db_connection,
    services::{
        auth,
        device_service::{self, DeviceError, DeviceLoginInfo, DeviceQueryRequest, DeviceSession},
        file_service::{self, FileError, PushResponse},
        jwt,
    },
};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserDeleteRequest {
    pub uid: u64,
}

#[derive(Debug, Validate, Serialize, Deserialize)]
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

/*
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

*/

#[derive(Debug, Validate, Serialize, Deserialize)]
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

#[page_helper]
#[derive(Debug, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct UserQueryRequest {
    pub status: Option<String>,
    pub role: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResp {
    pub uid: u64,
    pub username: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserLoginIdent {
    #[serde(rename = "username")]
    Username(String),
    #[serde(rename = "email")]
    Email(String),
}

#[derive(Debug)]
pub enum UserIdent {
    Username(String),
    Email(String),
    Uid(u64),
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

impl Display for UserLoginIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserLoginIdent::Email(ident) => write!(f, "{}", ident),
            UserLoginIdent::Username(ident) => write!(f, "{}", ident),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("User not found with {0}")]
    NotFound(UserIdent),

    #[error("User '{0}' is not authorized to perform this action.")]
    Unauthorized(String),

    #[error("Username or email '{0}' is already taken")]
    IdentTaken(String),

    #[error("Invalid password or username provided.")]
    InvalidUsernamePassword,

    #[error("Validate failed")]
    Validation(HashMap<Cow<'static, str>, ValidationErrorsKind>),

    #[error("User account '{0}' is locked.")]
    AccountLocked(String),

    #[error("Email address '{0}' is already registered.")]
    EmailAlreadyRegistered(String),

    #[error("Password confirm failed")]
    ConfirmationPasswordMismatch,

    #[error("Failed to create user: {0}")]
    UserCreationFailed(String),

    #[error("User {0} login failed")]
    LoginFailed(String),

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    let username = username.chars();
    let mut result = Ok(());
    for ch in username {
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

pub async fn get_user(uid: u64) -> Result<users::Model, UserError> {
    let db = get_db_connection().await;
    get_user_db(&db, uid).await
}

pub async fn get_user_db(db: &DatabaseConnection, uid: u64) -> Result<users::Model, UserError> {
    let user = users::Entity::find_by_id(uid)
        .one(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| UserError::InternalError(format!("Get user {uid} failed: {err}")))?;

    user.ok_or(UserError::NotFound(UserIdent::Uid(uid)))
        .map(|mut user| {
            user.password = "hidden".to_string();
            user
        })
}

pub async fn sign_up(body: Arc<UserSignUpRequest>) -> Result<UserResp, UserError> {
    let db = get_db_connection().await;
    sign_up_db(&db, body).await
}

pub async fn sign_up_db(
    db: &DatabaseConnection,
    body: Arc<UserSignUpRequest>,
) -> Result<UserResp, UserError> {
    if let Err(err) = body.validate() {
        return Err(UserError::Validation(err.errors().clone()));
    } else if body.confirm_password != body.password {
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

    match user.insert(db).await {
        Ok(user) => Ok(UserResp {
            uid: user.uid,
            username: user.username,
            email: user.email,
        }),
        Err(err) => {
            if let Some(sqlerr) = err.sql_err()
                && let SqlErr::UniqueConstraintViolation(message) = sqlerr
                && message.contains("username")
            // TODO check message.
            {
                Err(UserError::IdentTaken(body.username.clone()))
            } else {
                Err(UserError::InternalError(format!(
                    "Insert user failed: {err}"
                )))
            }
        }
    }
}

/// Login
pub async fn login(
    payload: Arc<UserLoginRequest>,
    device_info: DeviceLoginInfo,
) -> Result<String, UserError> {
    if let Err(err) = payload.validate() {
        return Err(UserError::Validation(err.errors().clone()));
    }
    let db = get_db_connection().await;
    login_db(&db, payload, &device_info).await
}

pub async fn login_db(
    db: &DatabaseConnection,
    payload: Arc<UserLoginRequest>,
    device_info: &DeviceLoginInfo,
) -> Result<String, UserError> {
    log::info!("user {} attempt login", payload.ident);
    let user = match &payload.ident {
        UserLoginIdent::Username(username) => users::Entity::find()
            .filter(users::Column::Username.eq(username))
            .all(db)
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| {
                UserError::InternalError(format!("Query user by username failed: {err}"))
            })?,
        UserLoginIdent::Email(email) => users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .all(db)
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| {
                UserError::InternalError(format!("Query user by email failed: {err}"))
            })?,
    };

    let Some(user) = user.first() else {
        return Err(UserError::InvalidUsernamePassword);
    };

    auth::verify_password(&payload.password, &user.password).map_err(|err| match err {
        argon2::password_hash::Error::Password => UserError::InvalidUsernamePassword,
        _ => UserError::LoginFailed(payload.ident.to_string()),
    })?;

    // Register the device session before issuing the token: the jti must be
    // findable by access checks and revocable by logout.
    let jti = jwt::generate_jti();
    device_service::upsert_device_session_db(db, user.uid, &jti, device_info)
        .await
        .map_err(|err| match err {
            DeviceError::InternalError(msg) => UserError::InternalError(msg),
            DeviceError::NotFound => {
                UserError::InternalError("Device session not found".to_string())
            }
        })?;

    let expire_secs = Configuration::get_global_configuration()
        .cosmox
        .auth
        .token_expire_secs;
    let token = jwt::generate_jwt(
        &user.uid.to_string(),
        &jti,
        jwt::get_jwt_secret_key(),
        expire_secs,
    )
    .inspect_err(|err| log::error!("{err}"))
    .map_err(|_err| UserError::InternalError("Token generate error".to_string()))?;

    Ok(token)
}

/// Revoke every device session of the user (logout everywhere).
pub async fn logout_all(uid: u64) -> Result<(), UserError> {
    let db = get_db_connection().await;
    logout_all_db(&db, uid).await
}

pub async fn logout_all_db(db: &DatabaseConnection, uid: u64) -> Result<(), UserError> {
    device_service::logout_all_device_sessions_db(db, uid)
        .await
        .map_err(|err| match err {
            DeviceError::NotFound => UserError::NotFound(UserIdent::Uid(uid)),
            DeviceError::InternalError(msg) => UserError::InternalError(msg),
        })
}

/// List device sessions filtered by `params` (admin/audit view; the API
/// layer gates this behind the `User.Audit` permission).
pub async fn query_devices(
    params: Arc<DeviceQueryRequest>,
) -> Result<(Vec<DeviceSession>, Pagination), UserError> {
    let db = get_db_connection().await;
    query_devices_db(&db, params).await
}

pub async fn query_devices_db(
    db: &DatabaseConnection,
    params: Arc<DeviceQueryRequest>,
) -> Result<(Vec<DeviceSession>, Pagination), UserError> {
    device_service::query_device_sessions_db(db, params)
        .await
        .map_err(|err| match err {
            // The query path never raises `NotFound`; this arm exists only
            // to keep the match exhaustive.
            DeviceError::NotFound => UserError::InternalError("Device session not found".into()),
            DeviceError::InternalError(msg) => UserError::InternalError(msg),
        })
}

/// Log out one specific device session. `uid` is the owning user of `did`
/// (resolved by the caller); the two-column bound keeps the deletion
/// scoped to that owner regardless of caller privileges.
pub async fn logout_device(uid: u64, did: u64) -> Result<(), UserError> {
    let db = get_db_connection().await;
    logout_device_db(&db, uid, did).await
}

pub async fn logout_device_db(
    db: &DatabaseConnection,
    uid: u64,
    did: u64,
) -> Result<(), UserError> {
    device_service::logout_device_by_did_db(db, uid, did)
        .await
        .map_err(|err| match err {
            DeviceError::NotFound => UserError::NotFound(UserIdent::Uid(uid)),
            DeviceError::InternalError(msg) => UserError::InternalError(msg),
        })
}

pub async fn delete(uid: u64) -> Result<(), UserError> {
    let db = get_db_connection().await;
    delete_db(&db, uid).await
}

pub async fn delete_db(db: &DatabaseConnection, uid: u64) -> Result<(), UserError> {
    users::Entity::delete_by_id(uid)
        .exec(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map(|_| ())
        .map_err(|err| UserError::InternalError(format!("Delete user {uid} failed: {err}")))
}

pub async fn query(
    params: Arc<UserQueryRequest>,
) -> Result<(Vec<users::Model>, Pagination), UserError> {
    let db = get_db_connection().await;
    query_db(&db, params).await
}

pub async fn query_db(
    db: &DatabaseConnection,
    params: Arc<UserQueryRequest>,
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

    let paginator = select.paginate(db, params.page_size);
    let result = paginator
        .fetch_page(page)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| UserError::InternalError(format!("Query users failed: {err}")))?;
    let total = paginator
        .num_items()
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| UserError::InternalError(format!("Count users failed: {err}")))?;
    let pagination = Pagination::new(total, params.page_size, paginator.cur_page(), "");
    Ok((result, pagination))
}

pub async fn upload_user_avatar<S, E>(uid: u64, payload: S) -> Result<PushResponse, FileError>
where
    S: StreamExt<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Display,
{
    let db = get_db_connection().await;
    let mut profile_picture_path =
        PathBuf::from(&Configuration::get_global_configuration().cosmox.data.path);
    profile_picture_path.push("avatar");
    if !profile_picture_path.exists() {
        std::fs::create_dir_all(&profile_picture_path).map_err(|err| match err.kind() {
            io::ErrorKind::StorageFull => FileError::InsufficientStorage,
            _ => FileError::InternalError(format!("Failed to create avatar directory: {err}")),
        })?;
    }

    let push_response =
        file_service::push_item_octet_stream_with_path(payload, profile_picture_path).await?;

    let _ = users::Entity::update_many()
        .col_expr(
            users::Column::Avatar,
            Expr::value(Value::BigUnsigned(Some(push_response.pmid))),
        )
        .filter(users::Column::Uid.eq(uid))
        .exec(db.as_ref())
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| {
            FileError::InternalError(format!("Update user {uid} avatar failed: {err}"))
        })?;
    Ok(push_response)
}
