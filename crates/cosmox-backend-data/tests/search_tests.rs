//! Integration tests for `search_service`.

mod common;

use common::TestContext;
use common::helpers;

use cosmox_backend_data::services::resource_service::{self, ResourceAddRequest};
use cosmox_backend_data::services::search_service::{self, SearchRequest};
use cosmox_backend_data::services::tag_service;

#[tokio::test]
async fn test_search_by_keyword_in_name() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "search_user1").await;
    let lid = helpers::create_library(
        &ctx.db,
        "Search Lib 1",
        uid,
        "search_type",
        "cosmox_srch_test",
    )
    .await;

    resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "Star Wars Episode IV".into(),
            lid,
            description: Some("A long time ago in a galaxy far far away".into()),
            level: 1,
        },
    )
    .await
    .expect("add_resource failed");
    resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "Star Trek".into(),
            lid,
            description: Some("Space the final frontier".into()),
            level: 1,
        },
    )
    .await
    .expect("add_resource failed");

    let (results, _) = search_service::search_db(
        &ctx.db,
        SearchRequest {
            keyword: "Star Wars".into(),
            tags: None,
            lid: None,
            before_create_datetime: None,
            after_create_datetime: None,
            before_last_update_datetime: None,
            after_last_update_datetime: None,
            page: Some(0),
            page_size: 10,
            sort: None,
        },
    )
    .await
    .expect("search failed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, Some("Star Wars Episode IV".into()));
}

#[tokio::test]
async fn test_search_by_keyword_in_description() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "search_user2").await;
    let lid = helpers::create_library(
        &ctx.db,
        "Search Lib 2",
        uid,
        "search_type",
        "cosmox_srch_test",
    )
    .await;

    resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "Inception".into(),
            lid,
            description: Some("A dream within a dream".into()),
            level: 1,
        },
    )
    .await
    .expect("add_resource failed");

    let (results, _) = search_service::search_db(
        &ctx.db,
        SearchRequest {
            keyword: "dream within".into(),
            tags: None,
            lid: None,
            before_create_datetime: None,
            after_create_datetime: None,
            before_last_update_datetime: None,
            after_last_update_datetime: None,
            page: Some(0),
            page_size: 10,
            sort: None,
        },
    )
    .await
    .expect("search failed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, Some("Inception".into()));
}

#[tokio::test]
async fn test_search_with_lid_filter() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "search_user3").await;
    let lid_a =
        helpers::create_library(&ctx.db, "Lib A", uid, "search_type", "cosmox_srch_test").await;
    let lid_b =
        helpers::create_library(&ctx.db, "Lib B", uid, "search_type", "cosmox_srch_test").await;

    resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "Common Name".into(),
            lid: lid_a,
            description: None,
            level: 1,
        },
    )
    .await
    .expect("add_resource failed");
    resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "Common Name".into(),
            lid: lid_b,
            description: None,
            level: 1,
        },
    )
    .await
    .expect("add_resource failed");

    let (results, _) = search_service::search_db(
        &ctx.db,
        SearchRequest {
            keyword: "Common".into(),
            tags: None,
            lid: Some(lid_a),
            before_create_datetime: None,
            after_create_datetime: None,
            before_last_update_datetime: None,
            after_last_update_datetime: None,
            page: Some(0),
            page_size: 10,
            sort: None,
        },
    )
    .await
    .expect("search failed");
    assert_eq!(results.len(), 1, "should find only the one in Lib A");
}

#[tokio::test]
async fn test_search_with_tags_filter() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "search_user4").await;
    let lid = helpers::create_library(
        &ctx.db,
        "Search Lib 3",
        uid,
        "search_type",
        "cosmox_srch_test",
    )
    .await;

    let tgid = tag_service::add_tag_group_db(&ctx.db, "Genre".into())
        .await
        .expect("add_tag_group failed");
    let tid = tag_service::add_tag_db(&ctx.db, "SciFi".into(), tgid)
        .await
        .expect("add_tag failed");

    let rid_a = resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "SciFi Movie".into(),
            lid,
            description: None,
            level: 1,
        },
    )
    .await
    .expect("add_resource failed");
    let _rid_b = resource_service::add_resource_db(
        &ctx.db,
        ResourceAddRequest {
            name: "Drama Movie".into(),
            lid,
            description: None,
            level: 1,
        },
    )
    .await
    .expect("add_resource failed");

    resource_service::add_tags_for_resource_db(&ctx.db, rid_a, vec![tid])
        .await
        .expect("add_tags_for_resource failed");

    let (results, _) = search_service::search_db(
        &ctx.db,
        SearchRequest {
            keyword: "Movie".into(),
            tags: Some(vec!["SciFi".into()]),
            lid: None,
            before_create_datetime: None,
            after_create_datetime: None,
            before_last_update_datetime: None,
            after_last_update_datetime: None,
            page: Some(0),
            page_size: 10,
            sort: None,
        },
    )
    .await
    .expect("search failed");
    assert_eq!(
        results.len(),
        1,
        "should find only the resource tagged SciFi"
    );
    assert_eq!(results[0].name, Some("SciFi Movie".into()));
}

#[tokio::test]
async fn test_search_pagination() {
    let ctx = TestContext::new().await;
    let uid = helpers::create_user(&ctx.db, "search_user5").await;
    let lid = helpers::create_library(
        &ctx.db,
        "Search Lib 4",
        uid,
        "search_type",
        "cosmox_srch_test",
    )
    .await;

    for i in 0..10 {
        resource_service::add_resource_db(
            &ctx.db,
            ResourceAddRequest {
                name: format!("Paginated Resource {i}"),
                lid,
                description: None,
                level: 1,
            },
        )
        .await
        .expect("add_resource failed");
    }

    let (page0, pagination) = search_service::search_db(
        &ctx.db,
        SearchRequest {
            keyword: "Paginated".into(),
            tags: None,
            lid: None,
            before_create_datetime: None,
            after_create_datetime: None,
            before_last_update_datetime: None,
            after_last_update_datetime: None,
            page: Some(0),
            page_size: 4,
            sort: Some("rid".into()),
        },
    )
    .await
    .expect("search failed");
    assert_eq!(page0.len(), 4, "first page should have 4 results");
    assert_eq!(pagination.total_items, 10);
    assert_eq!(pagination.total_pages, 3);

    let (page1, _) = search_service::search_db(
        &ctx.db,
        SearchRequest {
            keyword: "Paginated".into(),
            tags: None,
            lid: None,
            before_create_datetime: None,
            after_create_datetime: None,
            before_last_update_datetime: None,
            after_last_update_datetime: None,
            page: Some(1),
            page_size: 4,
            sort: Some("rid".into()),
        },
    )
    .await
    .expect("search failed");
    assert_eq!(page1.len(), 4, "second page should have 4 results");
}

#[tokio::test]
async fn test_search_empty_result() {
    let ctx = TestContext::new().await;

    let (results, pagination) = search_service::search_db(
        &ctx.db,
        SearchRequest {
            keyword: "NonexistentXYZ".into(),
            tags: None,
            lid: None,
            before_create_datetime: None,
            after_create_datetime: None,
            before_last_update_datetime: None,
            after_last_update_datetime: None,
            page: Some(0),
            page_size: 10,
            sort: None,
        },
    )
    .await
    .expect("search failed");
    assert!(results.is_empty());
    assert_eq!(pagination.total_items, 0);
}
