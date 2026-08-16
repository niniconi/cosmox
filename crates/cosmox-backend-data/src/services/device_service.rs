use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use common::message::Pagination;
use cosmox_macros::page_helper;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use serde::Deserialize;

use crate::entities::devices;
use crate::get_db_connection;

/// Errors related to device session management.
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("Device session not found")]
    NotFound,

    #[error("Internal service error: {0}")]
    InternalError(String),
}

/// Info captured at login for the device session record.
#[derive(Debug, Clone)]
pub struct DeviceLoginInfo {
    pub user_agent: Option<String>,
    pub ip: String,
}

/// Record a login: reuse the existing session row for the same user+UA
/// (replacing its jti forces the previous session out), or insert a new row.
pub async fn upsert_device_session(
    uid: u64,
    jti: &str,
    info: &DeviceLoginInfo,
) -> Result<(), DeviceError> {
    let db = get_db_connection().await;
    upsert_device_session_db(&db, uid, jti, info).await
}

pub async fn upsert_device_session_db(
    db: &DatabaseConnection,
    uid: u64,
    jti: &str,
    info: &DeviceLoginInfo,
) -> Result<(), DeviceError> {
    let now = Utc::now().naive_utc();

    if let Some(user_agent) = &info.user_agent {
        let existing = devices::Entity::find()
            .filter(devices::Column::LoginByUid.eq(uid))
            .filter(devices::Column::UserAgent.eq(user_agent))
            .one(db)
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| DeviceError::InternalError(format!("Query device failed: {err}")))?;

        if let Some(existing) = existing {
            let mut model: devices::ActiveModel = existing.into();
            model.current_jti = Set(Some(jti.to_string()));
            model.last_login_datetime = Set(now);
            model.last_login_ip = Set(info.ip.clone());
            model
                .update(db)
                .await
                .inspect_err(|err| log::error!("{err}"))
                .map_err(|err| {
                    DeviceError::InternalError(format!("Update device failed: {err}"))
                })?;
            return Ok(());
        }
    }

    let device = devices::ActiveModel {
        user_agent: Set(info.user_agent.clone()),
        login_by_uid: Set(Some(uid)),
        last_login_datetime: Set(now),
        last_login_ip: Set(info.ip.clone()),
        current_jti: Set(Some(jti.to_string())),
        ..Default::default()
    };
    device
        .insert(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| DeviceError::InternalError(format!("Insert device failed: {err}")))?;
    Ok(())
}

/// Non-sensitive device session info.
///
/// `uid` identifies the owning user so a cross-user query (`DeviceQueryRequest`
/// without a `uid` filter) can tell sessions apart; it serializes as the
/// `uid` field, mirroring `login_by_uid` in the database.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceSession {
    pub did: u64,
    pub uid: Option<u64>,
    pub user_agent: Option<String>,
    pub last_login_datetime: chrono::NaiveDateTime,
    pub last_login_ip: String,
}

/// Filter for listing device sessions of arbitrary users (admin/audit view).
///
/// With no `uid` the query returns sessions across all users; the API layer
/// gates the endpoint behind the `User.Audit` permission.
#[page_helper]
#[derive(Debug, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(bytecheck())]
pub struct DeviceQueryRequest {
    pub uid: Option<u64>,
}

/// List device sessions filtered by `params` (device management view).
pub async fn query_device_sessions(
    params: Arc<DeviceQueryRequest>,
) -> Result<(Vec<DeviceSession>, Pagination), DeviceError> {
    let db = get_db_connection().await;
    query_device_sessions_db(&db, params).await
}

pub async fn query_device_sessions_db(
    db: &DatabaseConnection,
    params: Arc<DeviceQueryRequest>,
) -> Result<(Vec<DeviceSession>, Pagination), DeviceError> {
    let mut select = devices::Entity::find();
    let mut page = 0;

    if let Some(inner_page) = params.page {
        page = inner_page;
    }

    if let Some(uid) = params.uid {
        select = select.filter(devices::Column::LoginByUid.eq(uid));
    }

    if let Some(sort) = &params.sort
        && let Ok(column) = devices::Column::from_str(sort)
    {
        select = select.order_by(column, sea_orm::Order::Asc);
    };

    let paginator = select.paginate(db, params.page_size);
    let sessions = paginator
        .fetch_page(page)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| DeviceError::InternalError(format!("Query devices failed: {err}")))?;
    let total = paginator
        .num_items()
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| DeviceError::InternalError(format!("Count devices failed: {err}")))?;
    let pagination = Pagination::new(total, params.page_size, paginator.cur_page(), "");

    Ok((
        sessions
            .into_iter()
            .map(|session| DeviceSession {
                did: session.did,
                uid: session.login_by_uid,
                user_agent: session.user_agent,
                last_login_datetime: session.last_login_datetime,
                last_login_ip: session.last_login_ip,
            })
            .collect(),
        pagination,
    ))
}

/// Resolve the owning user of device `did`.
///
/// Used to gate `LogoutDevice`: the access check compares this against the
/// requesting user and only lets self-service through without the
/// `User.SessionManage` permission.
pub async fn query_device_owner(did: u64) -> Result<Option<u64>, DeviceError> {
    let db = get_db_connection().await;
    query_device_owner_db(&db, did).await
}

pub async fn query_device_owner_db(
    db: &DatabaseConnection,
    did: u64,
) -> Result<Option<u64>, DeviceError> {
    let device = devices::Entity::find_by_id(did)
        .one(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| DeviceError::InternalError(format!("Query device failed: {err}")))?;

    Ok(device.and_then(|device| device.login_by_uid))
}

/// Delete the session `did`. The `uid` bound prevents a user from logging
/// out a session that does not belong to them.
pub async fn logout_device_by_did(uid: u64, did: u64) -> Result<(), DeviceError> {
    let db = get_db_connection().await;
    logout_device_by_did_db(&db, uid, did).await
}

pub async fn logout_device_by_did_db(
    db: &DatabaseConnection,
    uid: u64,
    did: u64,
) -> Result<(), DeviceError> {
    devices::Entity::delete_many()
        .filter(devices::Column::Did.eq(did))
        .filter(devices::Column::LoginByUid.eq(uid))
        .exec(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| DeviceError::InternalError(format!("Delete device failed: {err}")))?;
    Ok(())
}

/// Check whether `jti` belongs to an active session of `uid`.
///
/// Sessions are deleted on logout and on token expiry, so a missing row
/// means the token is no longer valid.
pub async fn validate_device_session(uid: u64, jti: &str) -> Result<bool, DeviceError> {
    let db = get_db_connection().await;
    validate_device_session_db(&db, uid, jti).await
}

pub async fn validate_device_session_db(
    db: &DatabaseConnection,
    uid: u64,
    jti: &str,
) -> Result<bool, DeviceError> {
    let device = devices::Entity::find()
        .filter(devices::Column::LoginByUid.eq(uid))
        .filter(devices::Column::CurrentJti.eq(jti))
        .one(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| DeviceError::InternalError(format!("Query device failed: {err}")))?;

    Ok(device.is_some())
}

/// Delete the session owning `jti`.
///
/// Used by logout and by the token-expiry cleanup path: the row is removed
/// so the session fully disappears from the device table.
pub async fn logout_device_session(jti: &str) -> Result<(), DeviceError> {
    let db = get_db_connection().await;
    logout_device_session_db(&db, jti).await
}

pub async fn logout_device_session_db(
    db: &DatabaseConnection,
    jti: &str,
) -> Result<(), DeviceError> {
    devices::Entity::delete_many()
        .filter(devices::Column::CurrentJti.eq(jti))
        .exec(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| DeviceError::InternalError(format!("Delete device failed: {err}")))?;
    Ok(())
}

/// Delete every session of `uid` (logout everywhere).
pub async fn logout_all_device_sessions(uid: u64) -> Result<(), DeviceError> {
    let db = get_db_connection().await;
    logout_all_device_sessions_db(&db, uid).await
}

pub async fn logout_all_device_sessions_db(
    db: &DatabaseConnection,
    uid: u64,
) -> Result<(), DeviceError> {
    devices::Entity::delete_many()
        .filter(devices::Column::LoginByUid.eq(uid))
        .exec(db)
        .await
        .inspect_err(|err| log::error!("{err}"))
        .map_err(|err| DeviceError::InternalError(format!("Delete devices failed: {err}")))?;
    Ok(())
}
