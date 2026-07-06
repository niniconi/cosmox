//! Integration tests for `resource_service`.

mod common;

use common::TestContext;
use common::helpers;

use cosmox_backend_data::services::resource_service::{
    self, ResourceAddRequest, ResourceError, ResourceQueryRequest,
};

#[tokio::test]
pub async fn add_and_get_resource() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "res_user").await;
    let lid = helpers::create_library(&ctx.db, "Res Lib", uid, "res_type", "cosmox_res_test").await;

    let rid = resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "test.mp4".into(),
            lid,
            description: Some("A test resource".into()),
            level: 0,
        },
    )
    .await
    .expect("add_resource failed");

    let resource = resource_service::get_resource_db(&ctx.db, rid)
        .await
        .expect("get_resource failed");
    assert_eq!(resource.name, Some("test.mp4".into()));
    assert_eq!(resource.rid, rid);
}

#[tokio::test]
pub async fn get_resource_not_found() {
    let ctx = TestContext::new().await;

    let err = resource_service::get_resource_db(&ctx.db, 99999)
        .await
        .unwrap_err();
    assert!(matches!(err, ResourceError::NotFound(99999)));
}

#[tokio::test]
pub async fn query_resources_by_lid() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "qry_user").await;
    let lid = helpers::create_library(&ctx.db, "Qry Lib", uid, "res_type", "cosmox_res_test").await;

    let rid_a = resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "a.mp4".into(),
            lid,
            description: None,
            level: 0,
        },
    )
    .await
    .expect("add_resource a failed");

    let _ = resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "b.mp4".into(),
            lid,
            description: None,
            level: 1,
        },
    )
    .await
    .expect("add_resource b failed");

    let (res, pagination) = resource_service::query_resources_db(
        &ctx.db,
        ResourceQueryRequest {
            lid,
            level: None,
            min_level: None,
            max_level: None,
            page: Some(0),
            page_size: 10,
            sort: Some("rid".into()),
        },
    )
    .await
    .expect("query_resources failed");
    assert_eq!(res.len(), 2, "should find 2 resources in library");
    assert_eq!(pagination.total_items, 2);

    // Query by exact level
    let (res, _) = resource_service::query_resources_db(
        &ctx.db,
        ResourceQueryRequest {
            lid,
            level: Some(0),
            min_level: None,
            max_level: None,
            page: Some(0),
            page_size: 10,
            sort: None,
        },
    )
    .await
    .expect("query_resources level=0 failed");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].rid, rid_a);
}

#[tokio::test]
pub async fn query_resources_by_level_range() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "range_user").await;
    let lid =
        helpers::create_library(&ctx.db, "Range Lib", uid, "res_type", "cosmox_res_test").await;

    let _ = resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "low.mp4".into(),
            lid,
            description: None,
            level: 1,
        },
    )
    .await
    .expect("add_resource low failed");

    let _ = resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "mid.mp4".into(),
            lid,
            description: None,
            level: 5,
        },
    )
    .await
    .expect("add_resource mid failed");

    let _ = resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "high.mp4".into(),
            lid,
            description: None,
            level: 10,
        },
    )
    .await
    .expect("add_resource high failed");

    // min_level..max_level
    let (res, _) = resource_service::query_resources_db(
        &ctx.db,
        ResourceQueryRequest {
            lid,
            level: None,
            min_level: Some(2),
            max_level: Some(8),
            page: Some(0),
            page_size: 10,
            sort: None,
        },
    )
    .await
    .expect("query_resources range failed");
    assert_eq!(res.len(), 1, "mid (level 5) should be the only match");
    assert_eq!(res[0].name, Some("mid.mp4".into()));
}

#[tokio::test]
pub async fn query_resources_invalid_level_range() {
    let ctx = TestContext::new().await;

    let err = resource_service::query_resources_db(
        &ctx.db,
        ResourceQueryRequest {
            lid: 1,
            level: None,
            min_level: Some(10),
            max_level: Some(5),
            page: Some(0),
            page_size: 10,
            sort: None,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, ResourceError::InvalidLevelRange));
}

#[tokio::test]
pub async fn query_resources_level_conflict() {
    let ctx = TestContext::new().await;

    let err = resource_service::query_resources_db(
        &ctx.db,
        ResourceQueryRequest {
            lid: 1,
            level: Some(1),
            min_level: Some(0),
            max_level: None,
            page: Some(0),
            page_size: 10,
            sort: None,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, ResourceError::LevelParameterConflict));
}

#[tokio::test]
pub async fn delete_resource_ok() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "del_user").await;
    let lid = helpers::create_library(&ctx.db, "Del Lib", uid, "res_type", "cosmox_res_test").await;

    let rid = resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "delete_me.mp4".into(),
            lid,
            description: None,
            level: 0,
        },
    )
    .await
    .expect("add_resource failed");

    resource_service::delete_resource_db(&ctx.db, rid)
        .await
        .expect("delete_resource failed");

    let err = resource_service::get_resource_db(&ctx.db, rid)
        .await
        .unwrap_err();
    assert!(matches!(err, ResourceError::NotFound(_)));
}
