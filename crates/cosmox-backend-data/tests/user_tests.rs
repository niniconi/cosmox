//! Integration tests for `user_service`.

mod common;

use std::sync::Arc;

use common::TestContext;

use cosmox_backend_data::services::user_service::{
    self, UserError, UserIdent, UserLoginIdent, UserLoginRequest, UserQueryRequest,
    UserSignUpRequest,
};

#[tokio::test]
pub async fn sign_up_and_get_user() {
    let ctx = TestContext::new().await;

    let req = Arc::new(UserSignUpRequest {
        username: "testuser".into(),
        nickname: Some("Test".into()),
        password: "Pass123!".into(),
        confirm_password: "Pass123!".into(),
        email: Some("test@example.com".into()),
    });
    let resp = user_service::sign_up_db(&ctx.db, req)
        .await
        .expect("sign_up failed");
    assert_eq!(resp.username, "testuser");
    assert_eq!(resp.email, Some("test@example.com".into()));
    assert!(resp.uid > 0);

    let user = user_service::get_user_db(&ctx.db, resp.uid)
        .await
        .expect("get_user failed");
    assert_eq!(user.username, "testuser");
}

#[tokio::test]
pub async fn sign_up_duplicate_username() {
    let ctx = TestContext::new().await;

    let req = Arc::new(UserSignUpRequest {
        username: "dupuser".into(),
        nickname: None,
        password: "Pass123!".into(),
        confirm_password: "Pass123!".into(),
        email: None,
    });
    user_service::sign_up_db(&ctx.db, req.clone())
        .await
        .expect("first sign_up failed");
    let err = user_service::sign_up_db(&ctx.db, req).await.unwrap_err();
    assert!(matches!(err, UserError::IdentTaken(_)));
}

#[tokio::test]
pub async fn sign_up_password_mismatch() {
    let ctx = TestContext::new().await;

    let req = Arc::new(UserSignUpRequest {
        username: "mismatch".into(),
        nickname: None,
        password: "Pass123!".into(),
        confirm_password: "Different!".into(),
        email: None,
    });
    // Validation fails before any DB call, so the original wrapper is safe here
    let err = user_service::sign_up_db(&ctx.db, req).await.unwrap_err();
    assert!(matches!(err, UserError::ConfirmationPasswordMismatch));
}

#[tokio::test]
pub async fn login_with_username() {
    let ctx = TestContext::new().await;

    let req = Arc::new(UserSignUpRequest {
        username: "loginuser".into(),
        nickname: None,
        password: "Pass123!".into(),
        confirm_password: "Pass123!".into(),
        email: Some("login@example.com".into()),
    });
    user_service::sign_up_db(&ctx.db, req)
        .await
        .expect("sign_up failed");

    let token = user_service::login_db(
        &ctx.db,
        Arc::new(UserLoginRequest {
            ident: UserLoginIdent::Username("loginuser".into()),
            password: "Pass123!".into(),
        }),
    )
    .await
    .expect("login failed");
    assert!(!token.is_empty(), "expected a JWT token");
}

#[tokio::test]
pub async fn login_with_email() {
    let ctx = TestContext::new().await;

    let req = Arc::new(UserSignUpRequest {
        username: "emaillogin".into(),
        nickname: None,
        password: "Pass123!".into(),
        confirm_password: "Pass123!".into(),
        email: Some("email-login@example.com".into()),
    });
    user_service::sign_up_db(&ctx.db, req)
        .await
        .expect("sign_up failed");

    let token = user_service::login_db(
        &ctx.db,
        Arc::new(UserLoginRequest {
            ident: UserLoginIdent::Email("email-login@example.com".into()),
            password: "Pass123!".into(),
        }),
    )
    .await
    .expect("login failed");
    assert!(!token.is_empty());
}

#[tokio::test]
pub async fn login_wrong_password() {
    let ctx = TestContext::new().await;

    let req = Arc::new(UserSignUpRequest {
        username: "badpwd".into(),
        nickname: None,
        password: "Pass123!".into(),
        confirm_password: "Pass123!".into(),
        email: None,
    });
    user_service::sign_up_db(&ctx.db, req)
        .await
        .expect("sign_up failed");

    let err = user_service::login_db(
        &ctx.db,
        Arc::new(UserLoginRequest {
            ident: UserLoginIdent::Username("badpwd".into()),
            password: "WrongPass!".into(),
        }),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, UserError::InvalidUsernamePassword));
}

#[tokio::test]
pub async fn delete_user() {
    let ctx = TestContext::new().await;

    let req = Arc::new(UserSignUpRequest {
        username: "deluser".into(),
        nickname: None,
        password: "Pass123!".into(),
        confirm_password: "Pass123!".into(),
        email: None,
    });
    let resp = user_service::sign_up_db(&ctx.db, req)
        .await
        .expect("sign_up failed");
    user_service::delete_db(&ctx.db, resp.uid)
        .await
        .expect("delete failed");

    let err = user_service::get_user_db(&ctx.db, resp.uid)
        .await
        .unwrap_err();
    assert!(matches!(err, UserError::NotFound(UserIdent::Uid(_))));
}

#[tokio::test]
pub async fn get_user_not_found() {
    let ctx = TestContext::new().await;

    let err = user_service::get_user_db(&ctx.db, 99999).await.unwrap_err();
    assert!(matches!(err, UserError::NotFound(UserIdent::Uid(99999))));
}

#[tokio::test]
pub async fn query_users_pagination() {
    let ctx = TestContext::new().await;

    for i in 0..5 {
        let req = Arc::new(UserSignUpRequest {
            username: format!("queryuser{i}"),
            nickname: None,
            password: "Pass123!".into(),
            confirm_password: "Pass123!".into(),
            email: None,
        });
        user_service::sign_up_db(&ctx.db, req)
            .await
            .expect("sign_up failed");
    }

    let (users, pagination) = user_service::query_db(
        &ctx.db,
        Arc::new(UserQueryRequest {
            status: None,
            role: None,
            search: None,
            page: Some(0),
            page_size: 3,
            sort: Some("uid".into()),
        }),
    )
    .await
    .expect("query failed");
    assert_eq!(users.len(), 3, "should return 3 users on page 0");
    assert_eq!(pagination.total_items, 5);
}

#[tokio::test]
pub async fn query_users_with_search() {
    let ctx = TestContext::new().await;

    for i in 0..3 {
        let req = Arc::new(UserSignUpRequest {
            username: format!("alpha{i}"),
            nickname: None,
            password: "Pass123!".into(),
            confirm_password: "Pass123!".into(),
            email: None,
        });
        user_service::sign_up_db(&ctx.db, req)
            .await
            .expect("sign_up failed");
    }
    let req = Arc::new(UserSignUpRequest {
        username: "beta0".into(),
        nickname: None,
        password: "Pass123!".into(),
        confirm_password: "Pass123!".into(),
        email: None,
    });
    user_service::sign_up_db(&ctx.db, req)
        .await
        .expect("sign_up failed");

    let (users, _) = user_service::query_db(
        &ctx.db,
        Arc::new(UserQueryRequest {
            status: None,
            role: None,
            search: Some("alpha".into()),
            page: Some(0),
            page_size: 10,
            sort: None,
        }),
    )
    .await
    .expect("query failed");
    assert_eq!(users.len(), 3, "should find 3 users matching 'alpha'");
    assert!(users.iter().all(|u| u.username.starts_with("alpha")));
}
