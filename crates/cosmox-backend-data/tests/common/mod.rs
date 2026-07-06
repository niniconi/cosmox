pub mod helpers;
mod setup;

use sea_orm::DatabaseConnection;
use url::Url;

pub struct TestContext {
    pub db: DatabaseConnection,
    pub base_url: Url,
    pub db_name: String,
}

impl Default for TestContext {
    fn default() -> Self {
        Self {
            db: Default::default(),
            base_url: Url::parse("https://placeholder.com").unwrap(),
            db_name: Default::default(),
        }
    }
}

impl TestContext {
    pub async fn new() -> Self {
        let base_url = Url::parse(
            &std::env::var("DATABASE_URL").expect("DATABASE_URL env var must be set for tests"),
        )
        .unwrap();

        let db_name = format!(
            "cosmox_test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );

        let db = setup::setup(&base_url, &db_name).await;

        Self {
            db,
            base_url,
            db_name,
        }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let base_url = self.base_url.clone();
        let db_name = self.db_name.clone();
        let future = setup::teardown(&base_url, &db_name);
        let result = std::thread::scope(|s| {
            s.spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build async-drop runtime");

                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rt.block_on(future)));

                result.map_err(|_| "async_drop panicked".to_owned())
            })
            .join()
            .unwrap_or_else(|_| Err("async_drop thread panicked".to_owned()))
        });

        if let Err(e) = result {
            panic!("{}", e);
        }
    }
}
