//! Shared helper functions for integration tests.

use std::path::PathBuf;
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use cosmox_backend_data::services::libraries_service::{self, LibraryAddRequest};
use cosmox_backend_data::services::user_service::{self, UserSignUpRequest};

/// Create a temporary directory for testing.
#[allow(dead_code)]
pub fn test_dir(root: &str, name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(root).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Sign up a temporary user and return their uid.
#[allow(dead_code)]
pub async fn create_user(db: &DatabaseConnection, name: &str) -> u64 {
    let req = Arc::new(UserSignUpRequest {
        username: name.into(),
        nickname: None,
        password: "Pass123!".into(),
        confirm_password: "Pass123!".into(),
        email: None,
    });
    user_service::sign_up_db(db, req)
        .await
        .expect("create_user failed")
        .uid
}

/// Create a media type by label and return its id.
#[allow(dead_code)]
pub(crate) async fn create_type(db: &DatabaseConnection, label: &str) -> u64 {
    libraries_service::add_media_types_db(db, vec![label.into()])
        .await
        .expect("add_media_types failed");
    let types = libraries_service::get_all_type_db(db)
        .await
        .expect("get_all_type failed");
    types.iter().find(|t| t.label == label).unwrap().tid
}

/// Create a type + library and return the library id.
#[allow(dead_code)]
pub async fn create_library(
    db: &DatabaseConnection,
    name: &str,
    uid: u64,
    type_label: &str,
    test_root: &str,
) -> u64 {
    let type_id = create_type(db, type_label).await;
    let lib_path = test_dir(test_root, name);
    let req = Arc::new(LibraryAddRequest {
        name: name.into(),
        description: None,
        r#type: type_id,
        tags: vec![],
        library_paths: vec![lib_path.to_string_lossy().into()],
    });
    let (lib, _, _) = libraries_service::create_library_with_tags_and_paths_db(db, req, uid)
        .await
        .expect("create library failed");
    lib.lid
}
