use cosmox_configuration::Configuration;
use migration::MigratorTrait;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Read,
    process::exit,
    sync::atomic::Ordering,
};
use sysinfo::System;

use crate::get_db_connection;

#[derive(Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub name: &'static str,
    pub version: &'static str,
    pub author: &'static str,
    pub is_first_boot: bool,
}

/// Errors related to system management operations (e.g., shutdown, restart).
#[derive(Debug, thiserror::Error)]
pub enum SystemError {
    #[error("System resource '{0}' not found.")]
    NotFound(String),

    #[error("Not authorized to perform system operation: {0}.")]
    Unauthorized(String),

    #[error("System is already in state '{0}'.")]
    AlreadyInState(String),

    #[error("System operation '{0}' failed: {1}")]
    OperationFailed(&'static str, String),

    #[error("Invalid system state for operation '{0}': current state is '{1}'.")]
    InvalidState(String, String),

    #[error("System configuration invalid: {0}")]
    ConfigurationError(String),

    #[error("System shutdown initiated.")]
    ShutdownInitiated, // Specific for async operations that might not immediately fail

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

pub async fn info() -> Result<SystemInfo, SystemError> {
    // let sys = System::new_all();
    let info = SystemInfo {
        os: System::name().unwrap_or(String::from("Unknown")),
        name: env!("PROJECT_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        author: env!("CARGO_PKG_AUTHORS"),
        is_first_boot: Configuration::get_global_configuration()
            .state
            .is_first_boot
            .load(Ordering::Relaxed),
    };
    Ok(info)
}

/// Restart server
pub async fn restart() -> Result<(), SystemError> {
    unimplemented!("Not implemented restart api")
}

pub async fn shutdown() -> ! {
    exit(0);
}

pub async fn about() -> Result<String, SystemError> {
    let mut readme = match File::open("./README.md") {
        Ok(file) => file,
        Err(err) => return Err(SystemError::OperationFailed("open", err.to_string())),
    };
    let metadata = match readme.metadata() {
        Ok(metadata) => metadata,
        Err(err) => return Err(SystemError::OperationFailed("metadata", err.to_string())),
    };
    let mut reuslt = String::with_capacity(metadata.len() as usize);
    match readme.read_to_string(&mut reuslt) {
        Ok(_) => Ok(reuslt),
        Err(err) => Err(SystemError::OperationFailed("read", err.to_string())),
    }
}

pub async fn log() -> Result<String, SystemError> {
    let log_path = &Configuration::get_global_configuration().cosmox.log.path;
    let log = fs::read_to_string(log_path);
    Ok(log.unwrap())
}

pub async fn delete_all() -> Result<(), SystemError> {
    let db = get_db_connection().await;
    migration::Migrator::fresh(db.as_ref())
        .await
        .inspect_err(|err| log::error!("Failed to reset database: {err}"))
        .map_err(|err| SystemError::OperationFailed("fresh", err.to_string()))?;

    if let Err(err) = std::fs::remove_file(".first_boot.lock") {
        log::error!("Failed to remove .first_boot.lock: {err}");
    }

    Configuration::get_global_configuration()
        .state
        .is_first_boot
        .store(true, Ordering::Relaxed);

    Ok(())
}
