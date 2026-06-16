use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

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
    pub state: StateConfiguration,
    pub proxy: ProxyConfiguration,
    pub qbittorrent: Option<QbittorrentServerConfiguration>,
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
    #[serde(default = "crate::default::default_data_path")]
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginConfiguration {
    #[serde(default = "crate::default::default_plugin_path")]
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CacheConfiguration {
    #[serde(default = "crate::default::default_cache_path")]
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogConfiguration {
    #[serde(default = "crate::default::default_log_path")]
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StateConfiguration {
    #[serde(default = "crate::default::default_state_path")]
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
    pub server: String,
    pub username: String,
    pub password: String,
}
