use std::{
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, atomic::Ordering},
};

use config::{Config as ConfigLoader, Environment, File};

use crate::default::default_config_path;

mod configuration;
mod default;

pub use configuration::{AuthConfiguration, Configuration, ScannerConfiguration};

/// Prefix of the environment variables able to override `application.yaml`.
///
/// The nesting level is expressed with a double underscore, so
/// `COSMX_SERVER__PORT=9090` overrides `server.port`.
pub const ENVIRONMENT_VARIABLE_PREFIX: &str = "COSMX";

/// Separator between two nesting levels of an environment variable name.
pub const ENVIRONMENT_VARIABLE_SEPARATOR: &str = "__";

static GLOBAL_CONFIGURATION: LazyLock<Configuration> = LazyLock::new(|| {
    let file = {
        let default_conf_file_path = PathBuf::from(default_config_path()).join("application.yaml");
        if let Ok(is_exists) = fs::exists(default_conf_file_path.as_path())
            && is_exists
            && let Some(path) = default_conf_file_path.to_str()
        {
            File::with_name(path).required(true)
        } else {
            File::with_name("application.yaml").required(true)
        }
    };
    // Environment variables are layered on top of the configuration file, so a
    // single image can be reconfigured without rebuilding it.
    let env = Environment::with_prefix(ENVIRONMENT_VARIABLE_PREFIX)
        .prefix_separator("_")
        .separator(ENVIRONMENT_VARIABLE_SEPARATOR)
        .convert_case(config::Case::Kebab)
        .try_parsing(true);
    let builder = ConfigLoader::builder().add_source(file).add_source(env);
    let config = match builder.build() {
        Ok(loader) => match loader.try_deserialize::<Configuration>() {
            Ok(config) => config,
            Err(err) => {
                // Config file exists but failed to parse: fail loudly, no silent fallback.
                panic!("failed to parse configuration: {err}")
            }
        },
        // Config file missing: fall back to the whole default configuration.
        // (When running outside a full deployment, /etc/cosmox-server may be absent
        // and the fallback is taken; in production the /etc path is checked first.)
        Err(err) => {
            log::error!("failed to load configuration, falling back to defaults: {err}");
            Configuration::default()
        }
    };

    config.state.is_first_boot.store(
        !Path::new(&config.cosmox.state.path)
            .join(".first_boot.lock")
            .exists(),
        Ordering::Relaxed,
    );
    config
});

impl Configuration {
    pub fn get_global_configuration() -> &'static Configuration {
        &GLOBAL_CONFIGURATION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{File as CfgFile, FileFormat};

    /// Environment overrides must win over the file values, including for
    /// fields whose yaml key is kebab-case while the env key is snake_case
    /// (`COSMX_COSMOX__SCANNER__METADATA_PATH` -> cosmox.scanner.metadata-path).
    #[test]
    fn env_override_wins_for_kebab_case_fields() {
        let yaml = r#"
server: { host: "127.0.0.1", port: 8080 }
database: { host: "localhost", port: 3306, user: "u", password: "p", database: "d" }
cosmox:
  name: "t"
  scanner: { metadata-path: "/file/metadata" }
  library: { path: "library" }
  data: { path: "data" }
  plugin: { path: "plugin" }
  cache: { path: "cache" }
  log: { path: "log" }
  state: { path: "state" }
  auth: { token-expire-secs: 100, refresh-threshold-secs: 10 }
  proxy: { http: "http://file:8889" }
"#;
        // Mutating process env vars is unsafe; these vars are restored immediately
        // after the check so they cannot leak into other tests.
        unsafe {
            std::env::set_var("COSMX_COSMOX__SCANNER__METADATA_PATH", "/env/metadata");
            std::env::set_var("COSMX_COSMOX__AUTH__TOKEN_EXPIRE_SECS", "200");
            std::env::set_var("COSMX_COSMOX__PROXY__HTTP", "http://env:8889");
        }
        let env = Environment::with_prefix(ENVIRONMENT_VARIABLE_PREFIX)
            .prefix_separator("_")
            .separator(ENVIRONMENT_VARIABLE_SEPARATOR)
            .convert_case(config::Case::Kebab)
            .try_parsing(true);
        let config = ConfigLoader::builder()
            .add_source(CfgFile::from_str(yaml, FileFormat::Yaml))
            .add_source(env)
            .build()
            .unwrap()
            .try_deserialize::<Configuration>()
            .unwrap();
        unsafe {
            std::env::remove_var("COSMX_COSMOX__SCANNER__METADATA_PATH");
            std::env::remove_var("COSMX_COSMOX__AUTH__TOKEN_EXPIRE_SECS");
            std::env::remove_var("COSMX_COSMOX__PROXY__HTTP");
        }
        assert_eq!(config.cosmox.scanner.metadata_path, "/env/metadata");
        assert_eq!(config.cosmox.auth.token_expire_secs, 200);
        assert_eq!(
            config.cosmox.proxy.http_proxy.as_deref(),
            Some("http://env:8889")
        );
    }

    /// The yaml-side kebab-case keys must actually populate the fields
    /// (previously they were silently dropped, e.g. auth fell back to defaults
    /// and proxy was always `None`).
    #[test]
    fn yaml_kebab_keys_are_deserialized() {
        let yaml = r#"
server: { host: "127.0.0.1", port: 8080 }
database: { host: "localhost", port: 3306, user: "u", password: "p", database: "d" }
cosmox:
  name: "t"
  scanner: { metadata-path: "/file/metadata", max-threads: 4 }
  library: { path: "library" }
  data: { path: "data" }
  plugin: { path: "plugin" }
  cache: { path: "cache" }
  log: { path: "log" }
  state: { path: "state" }
  auth: { token-expire-secs: 100, refresh-threshold-secs: 10 }
  proxy: { http: "http://file:8889" }
"#;
        let config = ConfigLoader::builder()
            .add_source(CfgFile::from_str(yaml, FileFormat::Yaml))
            .build()
            .unwrap()
            .try_deserialize::<Configuration>()
            .unwrap();
        assert_eq!(config.cosmox.scanner.metadata_path, "/file/metadata");
        assert_eq!(config.cosmox.scanner.max_threads, Some(4));
        assert_eq!(config.cosmox.auth.token_expire_secs, 100);
        assert_eq!(config.cosmox.auth.refresh_threshold_secs, 10);
        assert_eq!(
            config.cosmox.proxy.http_proxy.as_deref(),
            Some("http://file:8889")
        );
    }
}
