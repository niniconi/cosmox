//! Integration tests for `tag_service`.

mod common;

use common::TestContext;

use cosmox_backend_data::services::tag_service::{
    self, TagError, TagGroupQueryRequest, TagQueryRequest,
};

#[tokio::test]
pub async fn add_and_get_tag_group() {
    let ctx = TestContext::new().await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "Genre".into())
        .await
        .expect("add_tag_group failed");

    let group = tag_service::get_tag_group_db(&ctx.db, tgid)
        .await
        .expect("get_tag_group failed");
    assert_eq!(group.text, "Genre");
    assert_eq!(group.tgid, tgid);
}

#[tokio::test]
pub async fn get_tag_group_not_found() {
    let ctx = TestContext::new().await;

    let err = tag_service::get_tag_group_db(&ctx.db, 9999)
        .await
        .unwrap_err();
    assert!(matches!(err, TagError::NotFound(9999)));
}

#[tokio::test]
pub async fn get_tag_group_by_label() {
    let ctx = TestContext::new().await;

    tag_service::add_tag_group_db(&ctx.db, "Language".into())
        .await
        .expect("add_tag_group failed");

    let group = tag_service::get_tag_group_by_label_db(&ctx.db, "Language".into())
        .await
        .expect("get_tag_group_by_label failed")
        .expect("should find group");
    assert_eq!(group.text, "Language");
}

#[tokio::test]
pub async fn add_tag_group_duplicate() {
    let ctx = TestContext::new().await;

    tag_service::add_tag_group_db(&ctx.db, "UniqueGroup".into())
        .await
        .expect("first add_tag_group should succeed");

    let err = tag_service::add_tag_group_db(&ctx.db, "UniqueGroup".into())
        .await
        .unwrap_err();
    assert!(
        matches!(&err, TagError::AlreadyExists(msg) if msg.contains("UniqueGroup")),
        "expected AlreadyExists('UniqueGroup'), got {err:?}"
    );
}

#[tokio::test]
pub async fn delete_tag_group() {
    let ctx = TestContext::new().await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "ToDelete".into())
        .await
        .expect("add_tag_group failed");
    tag_service::delete_tag_group_db(&ctx.db, tgid)
        .await
        .expect("delete_tag_group failed");

    let err = tag_service::get_tag_group_db(&ctx.db, tgid)
        .await
        .unwrap_err();
    assert!(matches!(err, TagError::NotFound(_)));
}

#[tokio::test]
pub async fn add_and_get_tag() {
    let ctx = TestContext::new().await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "Genre".into())
        .await
        .expect("add_tag_group failed");

    let tid = tag_service::add_tag_db(&ctx.db, "Action".into(), tgid)
        .await
        .expect("add_tag failed");

    let tag = tag_service::get_tag_db(&ctx.db, tid)
        .await
        .expect("get_tag failed");
    assert_eq!(tag.text, "Action");
    assert_eq!(tag.tgid, tgid);
}

#[tokio::test]
pub async fn get_tag_not_found() {
    let ctx = TestContext::new().await;

    let err = tag_service::get_tag_db(&ctx.db, 9999).await.unwrap_err();
    assert!(matches!(err, TagError::NotFound(9999)));
}

#[tokio::test]
pub async fn delete_tag() {
    let ctx = TestContext::new().await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "Genre".into())
        .await
        .expect("add_tag_group failed");
    let tid = tag_service::add_tag_db(&ctx.db, "ToDelete".into(), tgid)
        .await
        .expect("add_tag failed");

    tag_service::delete_tag_db(&ctx.db, tid)
        .await
        .expect("delete_tag failed");

    let err = tag_service::get_tag_db(&ctx.db, tid).await.unwrap_err();
    assert!(matches!(err, TagError::NotFound(_)));
}

#[tokio::test]
pub async fn get_tag_by_label_found() {
    let ctx = TestContext::new().await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "Genre".into())
        .await
        .expect("add_tag_group failed");
    tag_service::add_tag_db(&ctx.db, "Comedy".into(), tgid)
        .await
        .expect("add_tag failed");

    let tag = tag_service::get_tag_by_label_db(&ctx.db, "Comedy".into())
        .await
        .expect("get_tag_by_label failed")
        .expect("tag should exist");
    assert_eq!(tag.text, "Comedy");
    assert_eq!(tag.tgid, tgid);
}

#[tokio::test]
pub async fn get_tag_by_label_not_found() {
    let ctx = TestContext::new().await;

    let res = tag_service::get_tag_by_label_db(&ctx.db, "NonExistent".into())
        .await
        .expect("get_tag_by_label should not error");
    assert!(res.is_none(), "expected None for missing tag");
}

#[tokio::test]
pub async fn add_tag_duplicate() {
    let ctx = TestContext::new().await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "Genre".into())
        .await
        .expect("add_tag_group failed");

    tag_service::add_tag_db(&ctx.db, "SameTag".into(), tgid)
        .await
        .expect("first add_tag should succeed");

    let err = tag_service::add_tag_db(&ctx.db, "SameTag".into(), tgid)
        .await
        .unwrap_err();
    assert!(
        matches!(&err, TagError::AlreadyExists(msg) if msg.contains("SameTag")),
        "expected AlreadyExists for duplicate tag, got {err:?}"
    );
}

#[tokio::test]
pub async fn query_tag_groups_paginated() {
    let ctx = TestContext::new().await;

    for name in &["A", "B", "C"] {
        tag_service::add_tag_group_db(&ctx.db, name.to_string())
            .await
            .expect("add_tag_group failed");
    }

    let (groups, pagination) = tag_service::query_tag_group_db(
        &ctx.db,
        TagGroupQueryRequest {
            tgid: None,
            page: Some(0),
            page_size: 2,
            sort: Some("tgid".into()),
        },
    )
    .await
    .expect("query_tag_group failed");
    assert_eq!(groups.len(), 2);
    assert_eq!(pagination.total_items, 3);
}

#[tokio::test]
pub async fn query_tags_paginated() {
    let ctx = TestContext::new().await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "Genre".into())
        .await
        .expect("add_tag_group failed");
    for name in &["X", "Y", "Z"] {
        tag_service::add_tag_db(&ctx.db, name.to_string(), tgid)
            .await
            .expect("add_tag failed");
    }

    let (tags, pagination) = tag_service::query_tag_db(
        &ctx.db,
        TagQueryRequest {
            tid: None,
            page: Some(0),
            page_size: 2,
            sort: Some("tid".into()),
        },
    )
    .await
    .expect("query_tag failed");
    assert_eq!(tags.len(), 2);
    assert_eq!(pagination.total_items, 3);
}

#[tokio::test]
pub async fn query_catalog() {
    let ctx = TestContext::new().await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "Genre".into())
        .await
        .expect("add_tag_group failed");
    tag_service::add_tag_db(&ctx.db, "Action".into(), tgid)
        .await
        .expect("add_tag failed");
    tag_service::add_tag_db(&ctx.db, "Drama".into(), tgid)
        .await
        .expect("add_tag failed");

    let catalog = tag_service::query_catalog_db(&ctx.db)
        .await
        .expect("query_catalog failed");
    assert_eq!(catalog.len(), 1, "should have 1 group");
    assert_eq!(catalog[0].group.text, "Genre");
    assert_eq!(catalog[0].tags.len(), 2, "should have 2 tags in group");
}
