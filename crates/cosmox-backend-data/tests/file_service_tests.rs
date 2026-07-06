//! Integration tests for `file_service`.

use bytes::Bytes;
use futures_util::stream;

mod common;

use common::TestContext;

use cosmox_backend_data::services::file_service::{self, FileError};

#[tokio::test]
pub async fn push_and_pull_link() {
    let ctx = TestContext::new().await;

    let url = url::Url::parse("file:///tmp/test_file.bin").unwrap();

    let pmid = file_service::push_item_link_db(&ctx.db, url.clone())
        .await
        .expect("push_item_link failed");
    assert!(pmid > 0, "should return a valid pmid");

    let path = file_service::pull_item_by_named_file_db(&ctx.db, pmid)
        .await
        .expect("pull_item_by_named_file failed");
    assert_eq!(path.to_string_lossy(), "/tmp/test_file.bin");
}

#[tokio::test]
pub async fn pull_item_not_found() {
    let ctx = TestContext::new().await;

    let err = file_service::pull_item_by_named_file_db(&ctx.db, 99999)
        .await
        .unwrap_err();
    assert!(matches!(err, FileError::NotFound(99999)));
}

#[tokio::test]
pub async fn push_multiple_links() {
    let ctx = TestContext::new().await;

    let pmid_a =
        file_service::push_item_link_db(&ctx.db, url::Url::parse("file:///tmp/a.bin").unwrap())
            .await
            .expect("push a failed");

    let pmid_b =
        file_service::push_item_link_db(&ctx.db, url::Url::parse("file:///tmp/b.bin").unwrap())
            .await
            .expect("push b failed");

    assert_ne!(pmid_a, pmid_b, "each push should return a unique pmid");

    let path_a = file_service::pull_item_by_named_file_db(&ctx.db, pmid_a)
        .await
        .expect("pull a failed");
    assert_eq!(path_a.to_string_lossy(), "/tmp/a.bin");

    let path_b = file_service::pull_item_by_named_file_db(&ctx.db, pmid_b)
        .await
        .expect("pull b failed");
    assert_eq!(path_b.to_string_lossy(), "/tmp/b.bin");
}

#[tokio::test]
pub async fn push_http_link_stores_path() {
    let ctx = TestContext::new().await;

    let url = url::Url::parse("https://example.com/file.txt").unwrap();
    let pmid = file_service::push_item_link_db(&ctx.db, url)
        .await
        .expect("push_item_link failed");

    assert!(pmid > 0);
}

#[tokio::test]
pub async fn push_octet_stream_with_path() {
    let ctx = TestContext::new().await;
    let tmp = std::env::temp_dir().join("cosmox_octet_test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let data = b"Hello, World!";
    let stream = stream::iter(vec![Ok::<_, std::io::Error>(Bytes::from_static(data))]);

    let resp = file_service::push_item_octet_stream_with_path_db(&ctx.db, stream, &tmp)
        .await
        .expect("push_item_octet_stream_with_path_db failed");

    assert!(resp.pmid > 0, "should return a valid pmid");
    assert_eq!(resp.uploaded_size, data.len() as u64);

    let stored = file_service::pull_item_by_named_file_db(&ctx.db, resp.pmid)
        .await
        .expect("pull_item_by_named_file should find the stored path");
    assert!(
        stored.starts_with(&tmp),
        "stored path {:?} should be under {:?}",
        stored,
        tmp
    );
    assert!(
        stored.exists(),
        "stored file {:?} should exist on disk",
        stored
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
