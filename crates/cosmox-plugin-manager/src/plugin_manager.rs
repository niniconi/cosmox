use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex},
};

use anyhow::{Result, anyhow};
use bytes::Bytes;
use common::default_constants::ascii_letters_number_separators;
use cosmox_configuration::Configuration;
use futures_util::StreamExt;
use lru::LruCache;
use tokio::{fs::File, io::AsyncWriteExt};
use url::Url;
use wasmtime::component::ResourceTable;
use wasmtime::{Engine, Store};
use wasmtime_wasi_http::WasiHttpCtx;

pub use super::plugin_loader::bindings::cosmox::plugin::context as bindings_context;
pub use super::plugin_loader::bindings::cosmox::plugin::cosmox_types as bindings_cosmox_types;

use crate::{
    Plugin, WasmComponent,
    plugin_lifecycle::plugin_wasm_lifecycle,
    plugin_loader::{
        ComponentRunStates, CosmoxPluginData, PluginLoadError, bindings, finalize_dependency,
        load_builtin_plugins, load_external_plugins,
    },
};

use wasmtime_wasi::WasiCtxBuilder;

/// Errors related to plugin management and execution.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin '{0}' not found in the registry or disk.")]
    NotFound(String),

    #[error("Not authorized to manage plugins.")]
    Unauthorized,

    #[error("Plugin '{0}' is disabled.")]
    Disabled(String),

    #[error("Plugin '{0}' is already installed.")]
    AlreadyInstalled(String),

    #[error("Plugin failed to load: {0}")]
    LoadFailed(#[from] PluginLoadError),

    #[error("Wasm instantiation failed for '{0}': {1}")]
    WasmError(String, String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to download from {url}. Remote HTTP status: {status}. Reason: {reason}")]
    HttpTransportError {
        url: Url,
        status: u16,
        reason: String,
    },

    #[error("Network transport connection lost for {url}. Details: {details}")]
    NetworkTransportError { url: Url, details: String },

    #[error("Stream transport abort. Details: {details}")]
    StreamTransportError { details: String },

    #[error("Local storage I/O system layer failure: {0}")]
    FileSystemError(String),

    #[error("Security block: Malicious relative path detected in tarball: {0}")]
    PathTraversalAttack(String),

    #[error("Plugin validation failed: {0}")]
    InvalidPluginPackage(String),

    /// Indicates an unexpected server-side issue.
    #[error("Internal server error: {0}")]
    InternalError(String),
}

#[derive(Default, Debug, Clone)]
pub struct PluginManager {
    /// wasm runtime engine
    pub engine: Arc<Engine>,

    pub plugin_enable_count: usize,
    pub plugin_count: usize,

    pub plugin_names: HashMap<String, u64>,
    pub plugins: Vec<Option<Plugin>>,

    pub wasm_list: HashMap<u64, Arc<WasmComponent>>,

    /// ```rust
    /// let event: Event;
    /// evet.into_key();
    /// ```
    pub event_map_to_wasm_components: HashMap<cosmox_api::EventKey, Vec<Arc<WasmComponent>>>,

    // pub id_map_to_name: HashMap<u64, String>,
    plugin_autoincrement: u64,
    wasm_autoincrement: u64,

    pub supported_media_types: HashSet<String>,
}

static SHARE: LazyLock<Mutex<LruCache<u64, Store<ComponentRunStates>>>> = LazyLock::new(|| {
    log::info!(
        "[Thread {:?}] Initializing LRU Cache for Stores. Thread ID: {:?}",
        std::thread::current().id(),
        std::thread::current().id()
    );
    // Sets the maximum capacity of the LRU cache. For example, each thread retains a maximum of 10 Store instances.
    // If you need more Store instances to be active simultaneously on the same thread, you can adjust this number.
    Mutex::new(LruCache::new(std::num::NonZeroUsize::new(10).unwrap()))
});

static PLUGIN_MANAGER: LazyLock<Mutex<PluginManager>> = LazyLock::new(|| {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    config.async_support(true);

    let engine = Arc::new(Engine::new(&config).unwrap());
    let plugins = vec![None; u16::MAX as usize];

    Mutex::new(PluginManager {
        engine,
        plugins,
        plugin_autoincrement: 0,
        wasm_autoincrement: 0,
        ..Default::default()
    })
});

/// Plugin manager
impl PluginManager {
    #[inline]
    pub fn get_plugin_manager() -> Arc<PluginManager> {
        Arc::new(PLUGIN_MANAGER.lock().unwrap().clone())
    }

    /// Generate a plugin_id from plugin manager
    /// ```rust
    /// let plugin_id0 = PluginManager::get_plugin_autoincrement();
    /// let plugin_id1 = PluginManager::get_plugin_autoincrement();
    /// assert_ne!(plugin_id0, plugin_id1);
    /// ```
    #[inline]
    pub fn get_plugin_autoincrement() -> u64 {
        PLUGIN_MANAGER.lock().unwrap()._get_plugin_autoincrement()
    }

    fn _get_plugin_autoincrement(&mut self) -> u64 {
        self.plugin_autoincrement += 1;
        self.plugin_autoincrement
    }

    #[inline]
    pub fn insert_plugin_name(name: String, id: u64) {
        PLUGIN_MANAGER.lock().unwrap()._insert_plugin_name(name, id)
    }

    pub fn _insert_plugin_name(&mut self, name: String, id: u64) {
        self.plugin_names.insert(name, id);
    }

    /// Generate a wasm_id from plugin manager
    /// ```rust
    /// let wasm_id0 = PluginManager::get_wasm_autoincrement();
    /// let wasm_id1 = PluginManager::get_wasm_autoincrement();
    /// assert_ne!(wasm_id0, wasm_id1);
    /// ```
    #[inline]
    pub fn get_wasm_autoincrement() -> u64 {
        PLUGIN_MANAGER.lock().unwrap()._get_wasm_autoincrement()
    }

    fn _get_wasm_autoincrement(&mut self) -> u64 {
        self.wasm_autoincrement += 1;
        self.wasm_autoincrement
    }

    pub async fn init() {
        {
            let plugin_path = &Configuration::get_global_configuration().cosmox.plugin.path;

            let builtin_plugins = load_builtin_plugins();
            let external_plugins = load_external_plugins(plugin_path);

            let mut plugin_manager = PLUGIN_MANAGER.lock().unwrap();

            plugin_manager._init(builtin_plugins, external_plugins);

            log::info!("initialized plugin manager");
        } // Leaving the life cycle of `plugin_manager`

        if let Err(errors) = finalize_dependency() {
            log::warn!("finalize dependency errors, count = {}", errors.len())
        }

        // The lock of `plugin_manager` has been released
        // PluginManager::lifecycle_manager();
        // let local_set = LocalSet::new();
        // let fut = local_set
        //   .run_until(async {
        // tokio::task::spawn_local(PluginManager::lifecycle_manager()).await
        // });
        PluginManager::lifecycle_manager().await
    }

    fn _init(&mut self, builtin_plugins: Vec<Plugin>, external_plugins: Vec<Plugin>) {
        self.plugin_count += builtin_plugins.len() + external_plugins.len();
        if self.plugin_count > u16::MAX as usize {
            panic!(
                "Plugin count overflow: current count is {}, but the limit is {}",
                self.plugin_count,
                u16::MAX
            );
        }
        for plugin in builtin_plugins {
            let id = plugin.id() as usize;
            self.plugins[id] = Some(plugin);
        }

        for plugin in external_plugins {
            let id = plugin.id() as usize;
            self.plugins[id] = Some(plugin);
        }
    }

    fn init_wasm_store_fn(
        engine: Arc<Engine>,
        wasm_component: Arc<WasmComponent>,
    ) -> Store<ComponentRunStates> {
        // log::trace!("[Thread '{}' (ID: {:?})] Lazily initializing Store for ServiceKey {}. Cache size: {}",
        // current_thread_name, current_thread_id, task_service_key, cache_size);

        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();
        let state = ComponentRunStates {
            wasi_ctx: wasi,
            wasi_http_ctx: WasiHttpCtx::new(),
            resource_table: ResourceTable::new(),
            plugin_data: CosmoxPluginData {
                bind_events: Mutex::new(Vec::with_capacity(32)),
                plugin_id: wasm_component.plugin_id,
                wasm_id: wasm_component.id,
                name: wasm_component.name.clone(),
            },
        };
        Store::new(&engine, state)
    }

    async fn lifecycle_manager() {
        let (engine, wasm_list) = {
            let plugin_manager = PLUGIN_MANAGER.lock().unwrap();
            let wasm_list = plugin_manager.wasm_list.clone();
            let engine = plugin_manager.engine.clone();
            (engine, wasm_list)
        };

        log::debug!("engine config = {:#?}", engine.config());

        for (wasm_id, wasm_component) in wasm_list.iter() {
            log::debug!(
                "start lifetime manager for wasm ID:{wasm_id}, wasm_component:{wasm_component:#?}"
            );

            let mut store = {
                let mut cache = SHARE.lock().unwrap();
                cache.pop(wasm_id).unwrap_or_else(|| {
                    log::trace!("[Thread '{}' (ID: {:?})] Lazily initializing Store for ServiceKey {}. Cache size: {}",
                        std::thread::current().name().unwrap_or("unnamed"),
                        std::thread::current().id(),
                        wasm_id,
                        cache.len());
                    PluginManager::init_wasm_store_fn(engine.clone(),wasm_component.clone())
                })
            };

            let instance = bindings::PluginHostWorld::instantiate_async(
                &mut store,
                &wasm_component.component,
                &wasm_component.linker,
            )
            .await
            .unwrap();

            plugin_wasm_lifecycle(&mut store, instance).await;

            {
                let mut cache = SHARE.lock().unwrap();
                cache.put(*wasm_id, store);
            }
        }
    }

    async fn extract_plugin<P: AsRef<Path>>(archive_path: &P) -> Result<(), PluginError> {
        let dst_path = PathBuf::from(
            Configuration::get_global_configuration()
                .cosmox
                .plugin
                .path
                .as_str(),
        );

        cosmox_plugin_packager::unpack(archive_path, dst_path)
            .map_err(|err| PluginError::InvalidPluginPackage(err.to_string()))?;
        Ok(())
    }

    pub async fn install_plugin_from_url(url: Url) -> Result<(), PluginError> {
        let resp =
            reqwest::get(url.clone())
                .await
                .inspect_err(|err| log::error!("{err}"))
                .map_err(|err| PluginError::NetworkTransportError {
                    url: url.clone(),
                    details: err.to_string(),
                })?;

        if resp.status().is_success() {
            Self::install_plugin_from_stream(resp.bytes_stream()).await
        } else {
            Err(PluginError::HttpTransportError {
                url,
                status: resp.status().as_u16(),
                reason: resp.status().to_string(),
            })
        }
    }

    pub async fn install_plugin_from_stream<S, E>(mut payload: S) -> Result<(), PluginError>
    where
        S: StreamExt<Item = Result<Bytes, E>> + Unpin,
        E: std::fmt::Display,
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let tmp_path = PathBuf::from(format!("/tmp/tmp_plugin_{}.tar.gz", timestamp));

        let mut file = File::create(&tmp_path)
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| {
                PluginError::FileSystemError(format!("Failed to create tmp file: {}", err))
            })?;

        log::info!("Downloading plugin to staged path: {:?}", tmp_path);

        while let Some(chunk_result) = payload.next().await {
            let chunk = match chunk_result {
                Ok(chunk) => chunk,
                Err(err) => {
                    tokio::fs::remove_file(tmp_path)
                        .await
                        .inspect_err(|err| log::error!("{err}"))
                        .map_err(|err| PluginError::InternalError(err.to_string()))?;

                    return Err(PluginError::StreamTransportError {
                        details: err.to_string(),
                    });
                }
            };

            file.write_all(&chunk)
                .await
                .inspect_err(|err| log::error!("{err}"))
                .map_err(|err| {
                    PluginError::FileSystemError(format!("Failed to write chunk to disk: {}", err))
                })?;
        }

        file.flush()
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| {
                PluginError::FileSystemError(format!("Failed to flush file: {}", err))
            })?;

        log::info!("Plugin tarball successfully saved at {:?}", tmp_path);

        Self::extract_plugin(&tmp_path).await?;
        Ok(())
    }

    pub fn uninstall() {}

    pub fn check() {}

    #[inline]
    pub fn enable() {
        PLUGIN_MANAGER.lock().unwrap().plugin_enable_count += 1;
    }

    #[inline]
    pub fn disable() {
        PLUGIN_MANAGER.lock().unwrap().plugin_enable_count -= 1;
    }

    pub fn query_ui_extensions() {}

    /// dispatched
    pub fn event_dispatcher() {}

    /// Notify event
    pub async fn notify_all<F>(
        event: Arc<cosmox_api::Event>,
        event_context_provider: F,
    ) -> Result<()>
    where
        F: Fn(&mut Store<ComponentRunStates>) -> bindings_context::EventContext,
    {
        log::debug!("notify all message by event{event:?}");
        let current_task_name = "notify event";

        let (components_for_current_event, engine) = {
            let plugin_manager = PLUGIN_MANAGER.lock().unwrap();
            let engine = plugin_manager.engine.clone();
            let components_for_current_event = match plugin_manager
                .event_map_to_wasm_components
                .get(&event.into_key())
            {
                Some(event_map_to_wasm_components) => event_map_to_wasm_components.clone(),
                None => return Err(anyhow!("Not found any event listeners")),
            };
            (components_for_current_event, engine)
        };

        let payload = match event.encode() {
            Ok(payload) => payload,
            Err(err) => return Err(anyhow!("Event encode error by {err}")),
        };

        // let mut join_handles = Vec::with_capacity(components_for_current_event.len());
        for current_wasm_compoent in &components_for_current_event {
            // let current_wasm_compoent = current_wasm_compoent.clone();
            let task_service_key = current_wasm_compoent.id;
            let payload = payload.clone();
            let engine = engine.clone();

            // let handle = tokio::task::spawn_blocking(move || {
            // let payload = payload;
            let current_thread = std::thread::current();
            let current_thread_id = current_thread.id();
            let current_thread_name = current_thread.name().unwrap_or("unnamed");
            log::info!(
                "[Host] Task '{}' (Service {}) dispatched to thread '{}' (ID: {:?}).",
                current_task_name,
                task_service_key,
                current_thread_name,
                current_thread_id
            );

            log::warn!("get lock state is {}", SHARE.try_lock().is_ok());
            let mut store = {
                let mut wasm_store_caches = SHARE.lock().unwrap();
                wasm_store_caches.pop(&task_service_key).unwrap_or_else(|| {
                    PluginManager::init_wasm_store_fn(engine.clone(), current_wasm_compoent.clone())
                })
            };

            let instance = bindings::PluginHostWorld::instantiate_async(
                &mut store,
                &current_wasm_compoent.component,
                &current_wasm_compoent.linker,
            )
            .await
            .unwrap();
            log::debug!("call on_event wasm = {current_wasm_compoent:?}");

            let event_context = event_context_provider(&mut store);
            let result = instance
                .cosmox_plugin_host_notifier()
                .call_on_event(&mut store, payload.as_slice(), event_context)
                .await;

            let cache_size = {
                let mut wasm_store_caches = SHARE.lock().unwrap();
                wasm_store_caches.put(task_service_key, store);
                wasm_store_caches.len()
            };

            match result {
                Ok(plugin_result) => {
                    log::info!(
                        "[Host] Wasm Task '{}' (Service {}) completed on thread '{}' (ID: {:?}). Result: {:?}. Current cache size: {}",
                        current_task_name,
                        task_service_key,
                        current_thread_name,
                        current_thread_id,
                        plugin_result,
                        cache_size,
                    );
                }
                Err(err) => {
                    log::error!("{err}")
                }
            }
        }
        Ok(())
    }

    #[inline]
    pub fn register_wasm_component(wasm_id: u64, wasm_component: Arc<WasmComponent>) {
        PLUGIN_MANAGER
            .lock()
            .unwrap()
            .wasm_list
            .insert(wasm_id, wasm_component);
    }

    #[inline]
    pub fn bind_event_for_wasm(
        event: cosmox_api::EventKey,
        wasm_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin_manager = PLUGIN_MANAGER.lock().unwrap();
        if let Some(wasm_component) = plugin_manager.wasm_list.get(&wasm_id) {
            let wasm_component = wasm_component.clone();

            if let Some(target_event_bind_wasm_componets) =
                plugin_manager.event_map_to_wasm_components.get_mut(&event)
            {
                target_event_bind_wasm_componets.push(wasm_component);
            } else {
                plugin_manager
                    .event_map_to_wasm_components
                    .insert(event, vec![wasm_component]);
            }
        }
        Ok(())
    }

    #[inline]
    pub fn unbind_event_from_wasm(
        event: cosmox_api::EventKey,
        wasm_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin_manager = PLUGIN_MANAGER.lock().unwrap();
        if let Some(target_event_bind_wasm_componets) =
            plugin_manager.event_map_to_wasm_components.get_mut(&event)
        {
            for (idx, wasm_component) in target_event_bind_wasm_componets.iter().enumerate() {
                if wasm_component.id == wasm_id {
                    target_event_bind_wasm_componets.remove(idx);
                    break;
                }
            }
        }
        Ok(())
    }

    /// add media type to cosmox
    /// # Arguments
    /// - `media_types`: A vec of media types.
    /// # Returns
    /// - `Ok(())` Add media types successful.
    /// - `Err(bindings_cosmox_types::MediaTypeError)` If it fails the check
    pub async fn push_media_types(
        media_types: Vec<String>,
    ) -> Result<(), bindings_cosmox_types::MediaTypeError> {
        // Validate and update in-memory cache (holding lock briefly)
        {
            let mut plugin_manager = PLUGIN_MANAGER.lock().unwrap();
            for media_type in media_types.iter() {
                if media_type.len() > 32
                    || media_type
                        .chars()
                        .any(|x| !ascii_letters_number_separators().contains(x))
                {
                    return Err(bindings_cosmox_types::MediaTypeError::InvalidFormat(
                        format!(
                            r#"
            At `{media_type}`:
            The `media_type` length must not exceed 32 characters.
            It can only contain alphanumeric characters (A-Z, a-z, 0-9), as well as spaces, underscores, and hyphens.
            "#
                        ),
                    ));
                } else if plugin_manager.supported_media_types.contains(media_type) {
                    return Err(bindings_cosmox_types::MediaTypeError::AlreadyExists(
                        format!("Type `{media_type}` already exists."),
                    ));
                }
            }

            if media_types.is_empty() {
                return Ok(());
            }

            for media_type in &media_types {
                plugin_manager
                    .supported_media_types
                    .insert(media_type.clone());
            }
        }

        if let Err(err) =
            cosmox_backend_data::services::libraries_service::add_media_types(media_types).await
        {
            log::error!("Failed to persist media types: {err}");
        }

        Ok(())
    }

    /// Get wasm runtime engine
    /// ```
    /// get wasm engine
    /// ```rust
    /// let engine = PluginManager::get_wasm_engine();
    /// ```
    #[inline]
    pub fn get_wasm_engine() -> Arc<Engine> {
        PLUGIN_MANAGER.lock().unwrap().engine.clone()
    }

    /// get wasm list
    pub fn get_wasm_list() -> Arc<HashMap<u64, Arc<WasmComponent>>> {
        Arc::new(PLUGIN_MANAGER.lock().unwrap().wasm_list.clone())
    }

    pub fn get_supported_media_types() -> Arc<HashSet<String>> {
        Arc::new(PLUGIN_MANAGER.lock().unwrap().supported_media_types.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_manager_entity() {
        println!("{:#?}", PLUGIN_MANAGER.lock().unwrap());
    }
}
