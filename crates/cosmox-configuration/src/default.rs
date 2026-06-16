pub fn default_config_path() -> String {
    #[cfg(target_os = "windows")]
    return "C:\\ProgramData\\cosmox-server".to_string();
    #[cfg(target_os = "macos")]
    return "/Library/Application Support/cosmox-server".to_string();
    "/etc/cosmox-server".to_string()
}
pub fn default_data_path() -> String {
    #[cfg(target_os = "windows")]
    return "C:\\ProgramData\\cosmox-server\\data".to_string();
    #[cfg(target_os = "macos")]
    return "/Library/Application Support/cosmox-server/data".to_string();
    "/var/lib/cosmox-server".to_string()
}

pub fn default_plugin_path() -> String {
    #[cfg(target_os = "windows")]
    return "C:\\ProgramData\\cosmox-server\\plugins".to_string();
    #[cfg(target_os = "macos")]
    return "/Library/Application Support/cosmox-server/plugins".to_string();
    "/var/lib/cosmox-server/plugins".to_string()
}

pub fn default_cache_path() -> String {
    #[cfg(target_os = "windows")]
    return "C:\\Windows\\Temp\\cosmox-server".to_string();
    #[cfg(target_os = "macos")]
    return "/Library/Caches/cosmox-server".to_string();
    "/var/cache/cosmox-server".to_string()
}
pub fn default_log_path() -> String {
    #[cfg(target_os = "windows")]
    return "C:\\ProgramData\\cosmox-server\\logs".to_string();
    #[cfg(target_os = "macos")]
    return "/Library/Logs/cosmox-server".to_string();
    "/var/log/cosmox-server".to_string()
}

pub fn default_state_path() -> String {
    #[cfg(target_os = "windows")]
    return "C:\\ProgramData\\cosmox-server\\state".to_string();
    #[cfg(target_os = "macos")]
    return "/Library/Application Support/cosmox-server/state".to_string();
    "/var/lib/cosmox-server/state".to_string()
}
