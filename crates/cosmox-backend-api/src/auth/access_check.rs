use std::sync::{Arc, atomic::Ordering};

use chrono::Utc;
use cosmox_backend_data::{
    RequestUser, RequestUserInner,
    define::Permission,
    services::{
        device_service, jwt,
        role_permission_service::{self, AuthError},
    },
};
use cosmox_configuration::Configuration;

use crate::{Context, api::Endpoint};

fn check_permissions(
    endpoint: Endpoint,
    permissions: &[Permission],
    current_user: Option<u64>,
    // Owning user of the device targeted by a `LogoutDevice` request,
    // resolved by `check_resource_access` before the permission check.
    device_owner: Option<u64>,
) -> bool {
    let mut perm_iter = permissions.iter();
    match endpoint {
        Endpoint::UploadAvatar { uid } => {
            if current_user == Some(uid) {
                return true;
            }
            perm_iter.any(|x| x.name == "User.ManageProfile")
        }

        Endpoint::GetSystemLog => perm_iter.any(|x| x.name == "System.LogView"),
        Endpoint::GetSystemAbout => perm_iter.any(|x| x.name == "System.About"),
        Endpoint::SystemDeleteAll => perm_iter.any(|x| x.name == "System.Wipe"),
        Endpoint::SystemShutdown | Endpoint::SystemRestart => {
            perm_iter.any(|x| x.name == "System.Power")
        }

        Endpoint::InstallPlugin => perm_iter.any(|x| x.name == "Plugin.Install"),
        Endpoint::UninstallPlugin => perm_iter.any(|x| x.name == "Plugin.Uninstall"),
        Endpoint::EnablePlugin | Endpoint::DisablePlugin | Endpoint::PluginInfo => {
            perm_iter.any(|x| x.name == "Plugin.Manage")
        }

        Endpoint::Register => perm_iter.any(|x| x.name == "User.Create"),
        Endpoint::DeleteUser { .. } => {
            perm_iter.any(|x| x.name == "User.Delete" || x.name == "User.ManageProfile")
        }
        Endpoint::QueryUser | Endpoint::GetUser { .. } => perm_iter.any(|x| x.name == "User.View"),
        Endpoint::AddRoleForUser { .. } => perm_iter.any(|x| x.name == "User.ManageRoles"),

        // Session management ops are gated by the dedicated permission,
        // except when the target is the requesting user themselves (or one
        // of their own devices): self-service needs no privilege.
        Endpoint::LogoutUser { uid } => {
            if current_user == Some(uid) {
                return true;
            }
            perm_iter.any(|x| x.name == "User.SessionManage")
        }
        Endpoint::LogoutDevice { .. } => {
            if current_user == device_owner {
                return true;
            }
            perm_iter.any(|x| x.name == "User.SessionManage")
        }
        // Querying one's own sessions (`uid` filter matching the requester) is
        // self-service, like other user-scoped endpoints; querying a
        // different user or all users requires the audit permission.
        Endpoint::QueryDevices { uid } => {
            if let Some(uid) = uid
                && Some(uid) == current_user
            {
                return true;
            }
            perm_iter.any(|x| x.name == "User.Audit")
        }

        Endpoint::AddRole
        | Endpoint::DeleteRole { .. }
        | Endpoint::QueryRole
        | Endpoint::GetRole { .. }
        | Endpoint::GetRolesByUser { .. } => perm_iter.any(|x| x.name == "User.ManageRoles"),

        Endpoint::AddPermission
        | Endpoint::DeletePermission { .. }
        | Endpoint::QueryPermission
        | Endpoint::GetPermission { .. }
        | Endpoint::GetPermissionsByRole { .. }
        | Endpoint::GetPermissionsByUser { .. }
        | Endpoint::AddPermissionForRole { .. } => perm_iter.any(|x| x.name == "User.ManagePerms"),

        Endpoint::AddLibrary => perm_iter.any(|x| x.name == "Library.Create"),
        Endpoint::GetSubPath => perm_iter.any(|x| x.name == "Library.PathView"),
        Endpoint::DeleteLibrary { .. } => perm_iter.any(|x| x.name == "Library.Delete"),
        Endpoint::ModifyLibrary { .. } => perm_iter.any(|x| x.name == "Library.Modify"),
        Endpoint::GetLibrary { .. } | Endpoint::QueryLibrary | Endpoint::GetAllLibraryTypes => {
            perm_iter.any(|x| x.name == "Library.View")
        }

        Endpoint::Scan { .. } | Endpoint::ScanAll => perm_iter.any(|x| x.name == "Library.Scan"),

        Endpoint::GetMetadata { .. }
        | Endpoint::QueryMetadata
        | Endpoint::GetMetadataOfResource { .. } => perm_iter.any(|x| x.name == "Metadata.View"),

        Endpoint::AddTag
        | Endpoint::AddTagGroup
        | Endpoint::DeleteTag { .. }
        | Endpoint::DeleteTagGroup { .. } => perm_iter.any(|x| x.name == "Tag.Manage"),

        Endpoint::GetTag { .. }
        | Endpoint::GetTagGroup { .. }
        | Endpoint::QueryTag
        | Endpoint::QueryTagGroup
        | Endpoint::GetTagCatalog => perm_iter.any(|x| x.name == "Tag.View"),

        Endpoint::AddResource => perm_iter.any(|x| x.name == "Resource.Create"),
        Endpoint::DeleteResource { .. } => perm_iter.any(|x| x.name == "Resource.Delete"),
        Endpoint::AddTagForResource { .. } => perm_iter.any(|x| x.name == "Tag.Assign"),
        Endpoint::GetResource { .. } | Endpoint::QueryResource => {
            perm_iter.any(|x| x.name == "Resource.View")
        }

        Endpoint::Search => {
            perm_iter.any(|x| x.name == "Library.View" || x.name == "Resource.View")
        }

        Endpoint::ItemPull { .. } => perm_iter.any(|x| x.name == "Media.Download"),
        Endpoint::ItemPush => perm_iter.any(|x| x.name == "Media.Upload"),

        _ => true,
    }
}

pub async fn check_resource_access(ctx: &mut Context<'_>) -> Result<RequestUser, AuthError> {
    let endpoint = &ctx.access_ctx.endpoint;
    log::debug!("check resource access for endpoint {endpoint:?}");

    if matches!(endpoint, Endpoint::None) {
        log::error!("Endpoint::None is invalid - handler may have forgotten to set endpoint");
        return Err(AuthError::Forbidden);
    }

    let token = ctx.access_ctx.token.as_deref();

    let is_first_boot = Configuration::get_global_configuration()
        .state
        .is_first_boot
        .load(Ordering::Relaxed);

    let is_white_listed = match endpoint {
        Endpoint::Login | Endpoint::Static => true,
        Endpoint::GetSystemInfo if is_first_boot => true,
        Endpoint::Init => {
            if is_first_boot {
                true
            } else {
                return Err(AuthError::Forbidden);
            }
        }
        _ => false,
    };

    if is_white_listed {
        Ok(Arc::new(RequestUserInner {
            uid: None,
            roles: vec!["Anonymous".to_string()],
            permissions: vec![],
            jti: None,
        }))
    } else if let Some(token) = token {
        match jwt::verify_and_decode_jwt(token, jwt::get_jwt_secret_key()) {
            Ok(claims) => {
                let Ok(uid): Result<u64, _> = claims.sub.parse() else {
                    log::error!(
                        "Failed to parse claims sub: expected integer, got '{}'",
                        claims.sub
                    );
                    return Err(AuthError::Unauthorized("Invalid token claims".to_string()));
                };

                // The token must belong to a non-revoked device session.
                // This is what makes logout take effect immediately.
                let session_active = device_service::validate_device_session(uid, &claims.jti)
                    .await
                    .map_err(|err| {
                        log::error!("Failed to validate device session for user {uid}: {err}");
                        AuthError::InternalError("Internal error".to_string())
                    })?;
                if !session_active {
                    log::warn!(
                        "Rejected revoked/unknown session jti {jti}",
                        jti = claims.jti
                    );
                    return Err(AuthError::Unauthorized(
                        "Session has been revoked, please log in again".to_string(),
                    ));
                }

                log::info!("user: {uid} access api: {endpoint:?}");

                let roles = match role_permission_service::get_roles_by_user(uid).await {
                    Ok(role) => role,
                    Err(err) => {
                        log::error!("Failed to get roles for user {uid}: {err}");
                        return Err(AuthError::InternalError("Internal error".to_string()));
                    }
                };
                let permissions = match role_permission_service::get_permissions_by_user(uid).await
                {
                    Ok(permissions) => permissions,
                    Err(err) => {
                        log::error!("Failed to get permissions for user {uid}: {err}");
                        return Err(AuthError::InternalError("Internal error".to_string()));
                    }
                };

                // Resolve the owner of the device targeted by LogoutDevice
                // so the permission check below can tell self-service from
                // managing another user's device.
                let device_owner = match endpoint {
                    Endpoint::LogoutDevice { did } => device_service::query_device_owner(*did)
                        .await
                        .map_err(|err| {
                            log::error!("Failed to resolve owner of device {did}: {err}");
                            AuthError::InternalError("Internal error".to_string())
                        })?,
                    _ => None,
                };

                let perm_check =
                    check_permissions(endpoint.clone(), &permissions, Some(uid), device_owner);

                if perm_check {
                    // Sliding renewal: when the token's remaining lifetime
                    // drops below the threshold, mint a fresh token (same
                    // jti) and hand it to the adapter via the response
                    // header, so the client never has to log in again.
                    let auth_config = &Configuration::get_global_configuration().cosmox.auth;
                    let now = Utc::now().timestamp() as u64;
                    if claims.exp.saturating_sub(now) <= auth_config.refresh_threshold_secs {
                        let fresh_token = jwt::generate_jwt(
                            &claims.sub,
                            &claims.jti,
                            jwt::get_jwt_secret_key(),
                            auth_config.token_expire_secs,
                        )
                        .inspect_err(|err| log::error!("{err}"))
                        .map_err(|_| AuthError::InternalError("Token generate error".into()))?;
                        ctx.access_ctx
                            .new_token
                            .lock()
                            .unwrap()
                            .replace(fresh_token);
                    }

                    Ok(Arc::new(RequestUserInner {
                        uid: Some(uid),
                        roles: roles.iter().map(|x| x.name.clone()).collect(),
                        permissions: permissions.iter().map(|x| x.name.clone()).collect(),
                        jti: Some(claims.jti.clone()),
                    }))
                } else {
                    Err(AuthError::Forbidden)
                }
            }
            Err(err) => {
                log::warn!("JWT verification failed for endpoint {endpoint:?}: {err}");
                if jwt::is_token_expired(&err) {
                    // The token is gone for good: drop its device row so the
                    // session does not linger in the devices table.
                    if let Ok(claims) =
                        jwt::decode_claims_without_exp_check(token, jwt::get_jwt_secret_key())
                    {
                        let _ = device_service::logout_device_session(&claims.jti).await;
                    }
                    Err(AuthError::TokenExpired("Token has expired".to_string()))
                } else {
                    Err(AuthError::Unauthorized(
                        "Invalid or expired token".to_string(),
                    ))
                }
            }
        }
    } else {
        Err(AuthError::Unauthorized(String::default()))
    }
}
