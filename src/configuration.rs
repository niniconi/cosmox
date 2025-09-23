use std::sync::LazyLock;

use config::{Config as ConfigLoader, File};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
  #[serde(rename = "server")]
  pub server: ServerConfiguration,
  #[serde(rename = "database")]
  pub database: DatabaseConfiguration,
  #[serde(rename = "cosmox")]
  pub cosmox: CosmoxConfiguration,
}

static GLOBAL_CONFIGURATION: LazyLock<Configuration> = LazyLock::new(|| {
  ConfigLoader::builder()
    .add_source(File::with_name("application.yaml").required(true))
    .build()
    .unwrap()
    .try_deserialize::<Configuration>()
    .unwrap()
});

impl Configuration {
  pub fn get_global_configuration() -> &'static Configuration {
    &GLOBAL_CONFIGURATION
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseConfiguration {
  pub host: String,
  pub port: u16,
  pub user: String,
  pub password: String,
  pub database: String,
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
