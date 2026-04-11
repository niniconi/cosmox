use std::{
  path::Path,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use config::{Config as ConfigLoader, File};
use log::LevelFilter;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::OnceCell;

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
  #[serde(rename = "server")]
  pub server: ServerConfiguration,
  #[serde(rename = "database")]
  pub database: DatabaseConfiguration,
  #[serde(rename = "cosmox")]
  pub cosmox: CosmoxConfiguration,
  #[serde(skip)]
  pub state: State,
}

#[derive(Debug, Default)]
pub struct State {
  pub is_first_boot: AtomicBool,
  pub db_connection: Arc<DatabaseConnection>,
}

static GLOBAL_CONFIGURATION: OnceCell<Configuration> = OnceCell::const_new();

impl Configuration {
  pub async fn get_global_configuration() -> &'static Configuration {
    GLOBAL_CONFIGURATION
      .get_or_init(|| async {
        let mut config = ConfigLoader::builder()
          .add_source(File::with_name("application.yaml").required(true))
          .build()
          .unwrap()
          .try_deserialize::<Configuration>()
          .unwrap();

        let database_url = format!(
          "mysql://{}:{}@{}:{}/{}",
          config.database.user,
          config.database.password,
          config.database.host,
          config.database.port,
          config.database.database
        );

        let db_connection = {
          if let Some(database_options) = &config.database.option {
            let mut database_opt = ConnectOptions::new(database_url);

            database_opt
              .max_connections(database_options.max_connections)
              .min_connections(database_options.min_connections)
              .connect_timeout(Duration::from_secs(database_options.connect_timeout))
              .acquire_timeout(Duration::from_secs(database_options.acquire_timeout))
              .idle_timeout(Duration::from_secs(database_options.idle_timeout))
              .max_lifetime(Duration::from_secs(database_options.max_lifetime))
              .sqlx_logging_level(LevelFilter::Debug);

            Database::connect(database_opt).await.unwrap()
          } else {
            Database::connect(database_url).await.unwrap()
          }
        };

        let db_connection = Arc::new(db_connection);

        config.state.db_connection = db_connection;

        config
          .state
          .is_first_boot
          .store(!Path::new(".first_boot.lock").exists(), Ordering::Relaxed);
        config
      })
      .await
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseConfiguration {
  pub host: String,
  pub port: u16,
  pub user: String,
  pub password: String,
  pub database: String,
  pub option: Option<DatabaseOptions>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseOptions {
  #[serde(rename = "max-connections")]
  pub max_connections: u32,
  #[serde(rename = "min-connections")]
  pub min_connections: u32,
  #[serde(rename = "connect-timeout")]
  pub connect_timeout: u64,
  #[serde(rename = "acquire-timeout")]
  pub acquire_timeout: u64,
  #[serde(rename = "idle-timeout")]
  pub idle_timeout: u64,
  #[serde(rename = "max-lifetime")]
  pub max_lifetime: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfiguration {
  pub host: String,
  pub port: u16,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CosmoxConfiguration {
  pub name: String,
  pub scanner: ScannerConfiguration,
  pub library: LibraryConfiguration,
  pub data: DataConfiguration,
  pub plugin: PluginConfiguration,
  pub cache: CacheConfiguration,
  pub log: LogConfiguration,
  pub proxy: ProxyConfiguration,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScannerConfiguration {
  #[serde(rename = "metadata-path")]
  pub metadata_path: String,

  #[serde(rename = "max-threads")]
  pub max_threads: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LibraryConfiguration {
  pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataConfiguration {
  pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginConfiguration {
  pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CacheConfiguration {
  pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogConfiguration {
  pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProxyConfiguration {
  pub http_proxy: Option<String>,
  pub https_proxy: Option<String>,
  pub socks5_proxy: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QbittorrentServerConfiguration {
  pub remote_addrerss: String,
  pub username: String,
  pub passowrd: String,
}

#[derive(Debug, Error)]
pub enum ConfigurationErr {
  #[error("Error:")]
  Err(String),
}
