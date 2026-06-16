use std::{
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, atomic::Ordering},
};

use config::{Config as ConfigLoader, File};

use crate::default::default_config_path;

mod configuration;
mod default;

pub use configuration::{Configuration, ScannerConfiguration};

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
    let config = ConfigLoader::builder()
        .add_source(file)
        .build()
        .unwrap()
        .try_deserialize::<Configuration>()
        .unwrap();

    config
        .state
        .is_first_boot
        .store(!Path::new(".first_boot.lock").exists(), Ordering::Relaxed);
    config
});

impl Configuration {
    pub fn get_global_configuration() -> &'static Configuration {
        &GLOBAL_CONFIGURATION
    }
}
