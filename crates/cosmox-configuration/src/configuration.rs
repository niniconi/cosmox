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
    pub auth: AuthConfiguration,
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

/// JWT lifetime and sliding-renewal threshold.
///
/// `refresh_threshold_secs` must be strictly smaller than `token_expire_secs`:
/// a request carrying a token whose remaining lifetime is below the threshold
/// gets a fresh token back via the `X-New-Token` response header.
#[derive(Debug, Deserialize, Serialize)]
pub struct AuthConfiguration {
    #[serde(default = "crate::default::default_token_expire_secs")]
    pub token_expire_secs: u64,
    #[serde(default = "crate::default::default_refresh_threshold_secs")]
    pub refresh_threshold_secs: u64,
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

/// Whole-configuration fallback used when application.yaml is entirely
/// missing. Missing individual fields inside an existing file are covered
/// by the per-field `#[serde(default = ...)]` attributes instead.
///
/// The values mirror `application.yaml` so the two default sources never
/// drift apart; keep them in sync when editing either one.
impl Default for Configuration {
    fn default() -> Self {
        Self {
            server: ServerConfiguration {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            database: DatabaseConfiguration {
                host: "localhost".to_string(),
                port: 3306,
                user: "root".to_string(),
                password: "123456".to_string(),
                database: "media".to_string(),
                option: None,
            },
            cosmox: CosmoxConfiguration {
                name: "cosmox server".to_string(),
                scanner: ScannerConfiguration {
                    metadata_path: "test/metadata".to_string(),
                    max_threads: None,
                },
                library: LibraryConfiguration {
                    path: "test/library".to_string(),
                },
                data: DataConfiguration {
                    path: "test/data".to_string(),
                },
                plugin: PluginConfiguration {
                    path: "test/plugin".to_string(),
                },
                cache: CacheConfiguration {
                    path: "test/cache".to_string(),
                },
                log: LogConfiguration {
                    path: "test/log".to_string(),
                },
                state: StateConfiguration {
                    path: "test/state".to_string(),
                },
                auth: AuthConfiguration {
                    token_expire_secs: crate::default::default_token_expire_secs(),
                    refresh_threshold_secs: crate::default::default_refresh_threshold_secs(),
                },
                proxy: ProxyConfiguration {
                    http_proxy: Some("http://proxy.server:8889".to_string()),
                    https_proxy: Some("http://proxy.server:8889".to_string()),
                    socks5_proxy: None,
                },
                qbittorrent: Some(QbittorrentServerConfiguration {
                    server: "http://127.0.0.1:7070".to_string(),
                    username: "admin".to_string(),
                    password: "123456".to_string(),
                }),
            },
            state: State::default(),
        }
    }
}
