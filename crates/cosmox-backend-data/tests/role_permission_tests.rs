//! Integration tests for `role_permission_service`.

mod common;

use common::TestContext;

use cosmox_backend_data::services::role_permission_service::{
    self, AclError, PermissionAddRequest, RoleAddRequest,
};
use cosmox_backend_data::services::user_service::{self, UserSignUpRequest};

use std::sync::Arc;

#[tokio::test]
pub async fn add_and_get_role() {
    let ctx = TestContext::new().await;

    role_permission_service::add_role_db(
        &ctx.db,
        RoleAddRequest {
            name: "editor".into(),
            description: Some("Can edit content".into()),
        },
    )
    .await
    .expect("add_role failed");

    // Built-in roles exist (seed data), so query all and find ours
    let roles = role_permission_service::query_role_db(&ctx.db)
        .await
        .expect("query_role failed");
    let editor = roles.iter().find(|r| r.name == "editor");
    assert!(editor.is_some(), "editor role should exist");
}

#[tokio::test]
pub async fn get_role_not_found() {
    let ctx = TestContext::new().await;

    let err = role_permission_service::get_role_db(&ctx.db, 99999)
        .await
        .unwrap_err();
    assert!(matches!(err, AclError::NotFoundRole(99999)));
}

#[tokio::test]
pub async fn delete_role() {
    let ctx = TestContext::new().await;

    role_permission_service::add_role_db(
        &ctx.db,
        RoleAddRequest {
            name: "temp_role".into(),
            description: None,
        },
    )
    .await
    .expect("add_role failed");

    let roles = role_permission_service::query_role_db(&ctx.db)
        .await
        .expect("query_role failed");
    let role = roles.iter().find(|r| r.name == "temp_role").unwrap();

    role_permission_service::delete_role_db(&ctx.db, role.rid)
        .await
        .expect("delete_role failed");

    let err = role_permission_service::get_role_db(&ctx.db, role.rid)
        .await
        .unwrap_err();
    assert!(matches!(err, AclError::NotFoundRole(_)));
}

#[tokio::test]
pub async fn add_and_get_permission() {
    let ctx = TestContext::new().await;

    role_permission_service::add_permission_db(
        &ctx.db,
        PermissionAddRequest {
            name: "resource:read".into(),
            description: Some("Read resources".into()),
        },
    )
    .await
    .expect("add_permission failed");

    let perms = role_permission_service::query_permission_db(&ctx.db)
        .await
        .expect("query_permission failed");
    let p = perms.iter().find(|p| p.name == "resource:read");
    assert!(p.is_some(), "permission should exist");
}

#[tokio::test]
pub async fn get_permission_not_found() {
    let ctx = TestContext::new().await;

    let err = role_permission_service::get_permission_db(&ctx.db, 99999)
        .await
        .unwrap_err();
    assert!(matches!(err, AclError::NotFoundPermission(99999)));
}

#[tokio::test]
pub async fn delete_permission() {
    let ctx = TestContext::new().await;

    role_permission_service::add_permission_db(
        &ctx.db,
        PermissionAddRequest {
            name: "temp_perm".into(),
            description: None,
        },
    )
    .await
    .expect("add_permission failed");

    let perms = role_permission_service::query_permission_db(&ctx.db)
        .await
        .expect("query_permission failed");
    let p = perms.iter().find(|p| p.name == "temp_perm").unwrap();

    role_permission_service::delete_permission_db(&ctx.db, p.pid)
        .await
        .expect("delete_permission failed");

    let err = role_permission_service::get_permission_db(&ctx.db, p.pid)
        .await
        .unwrap_err();
    assert!(matches!(err, AclError::NotFoundPermission(_)));
}

#[tokio::test]
pub async fn add_permission_to_role() {
    let ctx = TestContext::new().await;

    role_permission_service::add_role_db(
        &ctx.db,
        RoleAddRequest {
            name: "viewer".into(),
            description: None,
        },
    )
    .await
    .expect("add_role failed");

    role_permission_service::add_permission_db(
        &ctx.db,
        PermissionAddRequest {
            name: "view".into(),
            description: None,
        },
    )
    .await
    .expect("add_permission failed");

    let roles = role_permission_service::query_role_db(&ctx.db)
        .await
        .unwrap();
    let role = roles.iter().find(|r| r.name == "viewer").unwrap();
    let perms = role_permission_service::query_permission_db(&ctx.db)
        .await
        .unwrap();
    let perm = perms.iter().find(|p| p.name == "view").unwrap();

    role_permission_service::add_permission_for_role_db(&ctx.db, perm.pid, role.rid)
        .await
        .expect("add_permission_for_role failed");

    let role_perms = role_permission_service::get_permissions_by_role_db(&ctx.db, role.rid)
        .await
        .expect("get_permissions_by_role failed");
    assert!(role_perms.iter().any(|p| p.pid == perm.pid));
}

#[tokio::test]
pub async fn add_role_to_user() {
    let ctx = TestContext::new().await;

    // Create a user
    let signup = Arc::new(UserSignUpRequest {
        username: "roleuser".into(),
        nickname: None,
        password: "Pass123!".into(),
        confirm_password: "Pass123!".into(),
        email: None,
    });
    let user = user_service::sign_up_db(&ctx.db, signup)
        .await
        .expect("sign_up failed");

    // Create a role
    role_permission_service::add_role_db(
        &ctx.db,
        RoleAddRequest {
            name: "subscriber".into(),
            description: None,
        },
    )
    .await
    .expect("add_role failed");

    let roles = role_permission_service::query_role_db(&ctx.db)
        .await
        .unwrap();
    let role = roles.iter().find(|r| r.name == "subscriber").unwrap();

    role_permission_service::add_role_for_user_db(&ctx.db, role.rid, user.uid)
        .await
        .expect("add_role_for_user failed");

    // Verify via get_roles_by_user
    let user_roles = role_permission_service::get_roles_by_user_db(&ctx.db, user.uid)
        .await
        .expect("get_roles_by_user failed");
    assert!(user_roles.iter().any(|r| r.rid == role.rid));
}

#[tokio::test]
pub async fn get_permissions_by_user() {
    let ctx = TestContext::new().await;

    let signup = Arc::new(UserSignUpRequest {
        username: "permuser".into(),
        nickname: None,
        password: "Pass123!".into(),
        confirm_password: "Pass123!".into(),
        email: None,
    });
    let user = user_service::sign_up_db(&ctx.db, signup)
        .await
        .expect("sign_up failed");

    role_permission_service::add_role_db(
        &ctx.db,
        RoleAddRequest {
            name: "power_user".into(),
            description: None,
        },
    )
    .await
    .expect("add_role failed");

    role_permission_service::add_permission_db(
        &ctx.db,
        PermissionAddRequest {
            name: "power:do".into(),
            description: None,
        },
    )
    .await
    .expect("add_permission failed");

    let roles = role_permission_service::query_role_db(&ctx.db)
        .await
        .unwrap();
    let role = roles.iter().find(|r| r.name == "power_user").unwrap();
    let perms = role_permission_service::query_permission_db(&ctx.db)
        .await
        .unwrap();
    let perm = perms.iter().find(|p| p.name == "power:do").unwrap();

    role_permission_service::add_permission_for_role_db(&ctx.db, perm.pid, role.rid)
        .await
        .expect("add_permission_for_role failed");
    role_permission_service::add_role_for_user_db(&ctx.db, role.rid, user.uid)
        .await
        .expect("add_role_for_user failed");

    let user_perms = role_permission_service::get_permissions_by_user_db(&ctx.db, user.uid)
        .await
        .expect("get_permissions_by_user failed");
    assert!(
        user_perms.iter().any(|p| p.pid == perm.pid),
        "user should have the permission via role"
    );
}
