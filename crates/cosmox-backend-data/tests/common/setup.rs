//! Database setup / teardown for integration tests.
//!
//! Each test gets its own MySQL database named `cosmox_test_<uuid>`
//! so tests can run in parallel without interfering.
//! The database is dropped automatically when `TestContext` is dropped.
//!
//! Uses only sea-orm.

use migration::MigratorTrait;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};
use url::Url;

/// The base URL **without** a database name — the caller supplies it.
/// Returns (admin_url, test_db_url).
fn build_urls(base_url: &Url, db_name: &str) -> (Url, Url) {
    // base_url = mysql://user:pass@host:port/db
    // Replace the last path segment with our test db name.
    // We also produce an "admin" URL pointing at the `mysql` system DB
    // so we can CREATE / DROP our test database.

    (
        base_url.join("mysql").unwrap(),
        base_url.join(db_name).unwrap(),
    )
}

/// Create a fresh MySQL database, run all migrations, and return a connection.
pub async fn setup(base_url: &Url, db_name: &str) -> DatabaseConnection {
    let (admin_url, test_url) = build_urls(base_url, db_name);

    // Connect via sea-orm to the admin database and CREATE DATABASE
    let admin_db = Database::connect(admin_url.as_str())
        .await
        .expect("setup: failed to connect to admin database");

    let create_sql = format!("CREATE DATABASE IF NOT EXISTS `{}`", db_name);
    admin_db
        .execute_raw(Statement::from_string(DatabaseBackend::MySql, create_sql))
        .await
        .expect("setup: failed to create test database");

    drop(admin_db);

    // Connect to the test database via sea-orm and run migrations
    let mut test_opt = ConnectOptions::new(test_url);
    test_opt.max_connections(5);
    let test_db = Database::connect(test_opt)
        .await
        .expect("setup: failed to connect to test database");

    migration::Migrator::up(&test_db, None)
        .await
        .expect("setup: migration::up() failed");

    test_db
}

/// Drop a test database by name.
pub async fn teardown(base_url: &Url, db_name: &str) {
    let (admin_url, _) = build_urls(base_url, db_name);

    let admin_db = match Database::connect(admin_url.as_str()).await {
        Ok(db) => db,
        Err(_) => return, // best-effort cleanup
    };

    let drop_sql = format!("DROP DATABASE IF EXISTS `{}`", db_name);
    let _ = admin_db
        .execute_raw(Statement::from_string(DatabaseBackend::MySql, drop_sql))
        .await;
}
