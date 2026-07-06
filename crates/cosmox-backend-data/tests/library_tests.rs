//! Integration tests for `libraries_service`.

mod common;

use std::sync::Arc;

use common::TestContext;
use common::helpers;

use cosmox_backend_data::services::libraries_service::{
    self, LibraryAddRequest, LibraryError, LibraryQueryRequest, ModifyLibraryRequest,
};
use cosmox_backend_data::services::tag_service;

#[tokio::test]
async fn create_library_basic() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "lib_basic").await;

    let root = helpers::test_dir("cosmox_lib_test", "create_library_basic");
    let path = root.join("media").to_string_lossy().to_string();
    std::fs::create_dir_all(root.join("media")).unwrap();

    let type_id = helpers::create_type(&ctx.db, "lib_type").await;
    let (lib, _tags, _paths) = libraries_service::create_library_with_tags_and_paths_db(
        &ctx.db,
        Arc::new(LibraryAddRequest {
            name: "Test Library".into(),
            description: Some("A test".into()),
            r#type: type_id,
            tags: vec![],
            library_paths: vec![path.clone()],
        }),
        uid,
    )
    .await
    .expect("create_library_with_tags_and_paths failed");
    assert_eq!(lib.name, Some("Test Library".into()));
    assert!(lib.lid > 0);
}

#[tokio::test]
async fn create_library_with_tags() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "lib_tags").await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "Genre".into())
        .await
        .expect("add_tag_group failed");
    let tid = tag_service::add_tag_db(&ctx.db, "Sci-Fi".into(), tgid)
        .await
        .expect("add_tag failed");

    let root = helpers::test_dir("cosmox_lib_test", "create_library_with_tags");
    let path = root.join("media").to_string_lossy().to_string();
    std::fs::create_dir_all(root.join("media")).unwrap();

    let type_id = helpers::create_type(&ctx.db, "lib_type").await;
    let (_lib, tags, _paths) = libraries_service::create_library_with_tags_and_paths_db(
        &ctx.db,
        Arc::new(LibraryAddRequest {
            name: "Tagged Library".into(),
            description: None,
            r#type: type_id,
            tags: vec![tid],
            library_paths: vec![path.clone()],
        }),
        uid,
    )
    .await
    .expect("create_library_with_tags_and_paths failed");
    assert!(!tags.is_empty(), "should have at least one tag relation");
}

#[tokio::test]
async fn get_library_found() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "lib_found").await;

    let root = helpers::test_dir("cosmox_lib_test", "get_library_found");
    let path = root.join("media").to_string_lossy().to_string();
    std::fs::create_dir_all(root.join("media")).unwrap();

    let type_id = helpers::create_type(&ctx.db, "lib_type").await;
    let (lib, _, _) = libraries_service::create_library_with_tags_and_paths_db(
        &ctx.db,
        Arc::new(LibraryAddRequest {
            name: "Findable".into(),
            description: None,
            r#type: type_id,
            tags: vec![],
            library_paths: vec![path.clone()],
        }),
        uid,
    )
    .await
    .expect("create_library failed");

    let fetched = libraries_service::get_library_db(&ctx.db, lib.lid)
        .await
        .expect("get_library failed");
    assert_eq!(fetched.name, Some("Findable".into()));
}

#[tokio::test]
async fn get_library_not_found() {
    let ctx = TestContext::new().await;

    let err = libraries_service::get_library_db(&ctx.db, 99999)
        .await
        .unwrap_err();
    assert!(matches!(err, LibraryError::NotFound(99999)));
}

#[tokio::test]
async fn modify_library_name() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "lib_modify").await;

    let root = helpers::test_dir("cosmox_lib_test", "modify_library_name");
    let path = root.join("media").to_string_lossy().to_string();
    std::fs::create_dir_all(root.join("media")).unwrap();

    let type_id = helpers::create_type(&ctx.db, "lib_type").await;
    let (lib, _, _) = libraries_service::create_library_with_tags_and_paths_db(
        &ctx.db,
        Arc::new(LibraryAddRequest {
            name: "Original".into(),
            description: None,
            r#type: type_id,
            tags: vec![],
            library_paths: vec![path.clone()],
        }),
        uid,
    )
    .await
    .expect("create_library failed");

    libraries_service::modify_library_db(
        &ctx.db,
        lib.lid,
        ModifyLibraryRequest {
            name: Some("Updated".into()),
            description: None,
        },
    )
    .await
    .expect("modify_library failed");

    let updated = libraries_service::get_library_db(&ctx.db, lib.lid)
        .await
        .expect("get_library failed");
    assert_eq!(updated.name, Some("Updated".into()));
}

#[tokio::test]
async fn delete_library_ok() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "lib_delete").await;

    let root = helpers::test_dir("cosmox_lib_test", "delete_library_ok");
    let path = root.join("media").to_string_lossy().to_string();
    std::fs::create_dir_all(root.join("media")).unwrap();

    let type_id = helpers::create_type(&ctx.db, "lib_type").await;
    let (lib, _, _) = libraries_service::create_library_with_tags_and_paths_db(
        &ctx.db,
        Arc::new(LibraryAddRequest {
            name: "ToDelete".into(),
            description: None,
            r#type: type_id,
            tags: vec![],
            library_paths: vec![path.clone()],
        }),
        uid,
    )
    .await
    .expect("create_library failed");

    libraries_service::delete_library_db(&ctx.db, lib.lid)
        .await
        .expect("delete_library failed");

    let err = libraries_service::get_library_db(&ctx.db, lib.lid)
        .await
        .unwrap_err();
    assert!(matches!(err, LibraryError::NotFound(_)));
}

#[tokio::test]
async fn add_media_types_and_get_all() {
    let ctx = TestContext::new().await;

    let types_before = libraries_service::get_all_type_db(&ctx.db).await.unwrap();
    let count_before = types_before.len();

    libraries_service::add_media_types_db(&ctx.db, vec!["test_video".into(), "test_audio".into()])
        .await
        .expect("add_media_types failed");

    let types_after = libraries_service::get_all_type_db(&ctx.db).await.unwrap();
    assert_eq!(
        types_after.len(),
        count_before + 2,
        "should have 2 more types"
    );
}

#[tokio::test]
async fn add_media_types_idempotent() {
    let ctx = TestContext::new().await;

    libraries_service::add_media_types_db(&ctx.db, vec!["dup_type".into()])
        .await
        .expect("first add failed");
    libraries_service::add_media_types_db(&ctx.db, vec!["dup_type".into()])
        .await
        .expect("second add (on conflict do nothing) should succeed");
}

#[tokio::test]
async fn query_libraries_empty() {
    let ctx = TestContext::new().await;

    let (libs, pagination) = libraries_service::query_libraries_db(
        &ctx.db,
        LibraryQueryRequest {
            page: Some(0),
            page_size: 10,
            sort: None,
        },
    )
    .await
    .expect("query_libraries failed");
    assert!(libs.is_empty());
    assert_eq!(pagination.total_items, 0);
}

#[tokio::test]
async fn add_tags_for_library() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "lib_addtag").await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "Genre".into())
        .await
        .expect("add_tag_group failed");
    let tid = tag_service::add_tag_db(&ctx.db, "Action".into(), tgid)
        .await
        .expect("add_tag failed");

    let root = helpers::test_dir("cosmox_lib_test", "add_tags_for_library");
    let path = root.join("media").to_string_lossy().to_string();
    std::fs::create_dir_all(root.join("media")).unwrap();

    let type_id = helpers::create_type(&ctx.db, "lib_type").await;
    let (lib, _, _) = libraries_service::create_library_with_tags_and_paths_db(
        &ctx.db,
        Arc::new(LibraryAddRequest {
            name: "TagMe".into(),
            description: None,
            r#type: type_id,
            tags: vec![],
            library_paths: vec![path.clone()],
        }),
        uid,
    )
    .await
    .expect("create_library failed");

    let tag_rels = libraries_service::add_tags_for_library_db(&ctx.db, lib.lid, vec![tid])
        .await
        .expect("add_tags_for_library failed");
    assert_eq!(tag_rels.len(), 1);
}
