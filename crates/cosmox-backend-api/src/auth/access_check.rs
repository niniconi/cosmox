use std::sync::{Arc, atomic::Ordering};

use cosmox_backend_data::{
    RequestUser, RequestUserInner,
    define::Permission,
    services::{
        auth,
        role_permission_service::{self, AuthError},
    },
};
use cosmox_configuration::Configuration;

use crate::{Context, api::Endpoint};

fn check_permissions(
    endpoint: Endpoint,
    permissions: &[Permission],
    current_user: Option<u64>,
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

pub async fn check_resource_access(ctx: &Context<'_>) -> Result<RequestUser, AuthError> {
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
        }))
    } else if let Some(token) = token {
        match auth::verify_and_decode_jwt(token, auth::get_jwt_secret_key()) {
            Ok(claims) => {
                let Ok(uid): Result<u64, _> = claims.sub.parse() else {
                    log::error!(
                        "Failed to parse claims sub: expected integer, got '{}'",
                        claims.sub
                    );
                    return Err(AuthError::Unauthorized("Invalid token claims".to_string()));
                };

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

                let perm_check = check_permissions(endpoint.clone(), &permissions, Some(uid));

                if perm_check {
                    Ok(Arc::new(RequestUserInner {
                        uid: Some(uid),
                        roles: roles.iter().map(|x| x.name.clone()).collect(),
                        permissions: permissions.iter().map(|x| x.name.clone()).collect(),
                    }))
                } else {
                    Err(AuthError::Forbidden)
                }
            }
            Err(_) => {
                log::warn!("JWT verification failed for endpoint {endpoint:?}");
                Err(AuthError::Unauthorized(
                    "Invalid or expired token".to_string(),
                ))
            }
        }
    } else {
        Err(AuthError::Unauthorized(String::default()))
    }
}
