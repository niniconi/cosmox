use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use anyhow::{Result, anyhow};
use bytes::Bytes;
use common::default_constants::ascii_letters_number_separators;
use cosmox_configuration::Configuration;
use futures_util::StreamExt;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use url::Url;
use wasmtime::component::ResourceTable;
use wasmtime::{Engine, Store};
use wasmtime_wasi_http::WasiHttpCtx;

pub use super::plugin_loader::bindings::cosmox::plugin::context as bindings_context;
pub use super::plugin_loader::bindings::cosmox::plugin::cosmox_types as bindings_cosmox_types;

use crate::{
    plugin_lifecycle::plugin_wasm_lifecycle,
    plugin_loader::{
        ComponentRunStates, CosmoxPluginData, PluginLoadError, PluginLoader, bindings,
    },
    types::{LazyLoadPlugin, PluginName, PluginWasmId, PluginWasmName, WasmComponent},
};

use wasmtime_wasi::WasiCtxBuilder;

/// Errors related to plugin management and execution.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin '{0}' not found in the plugin manager.")]
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PluginManagerState {
    enabled_plugins: Vec<PluginState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginState {
    pub name: PluginName,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginWasmState {
    pub name: PluginWasmName,
}

/// Item returned by the plugin query API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginQueryItem {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub email: String,
    pub enabled: bool,
    /// "builtin" | "external" | "invalid"
    pub plugin_type: String,
    /// Error message when the plugin is in invalid state.
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PluginManager {
    /// wasm runtime engine
    pub engine: Engine,

    pub supported_media_types: HashSet<String>,

    pub plugin_loader: PluginLoader,

    pub stat: PluginManagerState,
}

static SHARE: LazyLock<Mutex<LruCache<PluginWasmId, Store<ComponentRunStates>>>> =
    LazyLock::new(|| {
        log::info!(
            "[Thread {:?}] Initializing LRU Cache for Stores. Thread ID: {:?}",
            std::thread::current().id(),
            std::thread::current().id()
        );
        // Sets the maximum capacity of the LRU cache. For example, each thread retains a maximum of 10 Store instances.
        // If you need more Store instances to be active simultaneously on the same thread, you can adjust this number.
        Mutex::new(LruCache::new(std::num::NonZeroUsize::new(10).unwrap()))
    });

static PLUGIN_MANAGER: LazyLock<RwLock<PluginManager>> = LazyLock::new(|| {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    config.async_support(true);

    let engine = Engine::new(&config).unwrap();
    let plugin_loader = PluginLoader::new(engine.clone());

    RwLock::new(PluginManager {
        engine,
        plugin_loader,
        supported_media_types: HashSet::<String>::new(),
        stat: PluginManagerState::default(),
    })
});

/// Plugin Manager
///
/// Plugin Manager lifecycle:
///
/// ```text
///   uninit
///     │
///     ▼
///   load_plugin_manager_stat ──────► read persisted state
///     │
///     ▼
///   _init { load_enabled_plugins }
///                     │
///                     ├──► discover plugins (filesystem + builtin)
///                     ├──► filter by enabled_plugins
///                     ├──► alloc plugin IDs
///                     ├──► finalize_dependency_all
///                     │       ├──► resolve deps / conflicts
///                     │       └──► build reverse_dep map
///                     ├──► dependencies_analyzer
///                     │       └──► topological sort (Kahn) ──► parallel batches
///                     └──► for each batch:
///                             ├──► load_wasm (+ jit compile) ──► external
///                             ├──► run ──────► builtin
///                             └──► register_plugin (bind events)
///     │
///     ▼
///   lifecycle_manager
///          │
///          └──► for each wasm_component:
///                  ├──► init_wasm_store (or pull from SHARE cache)
///                  ├──► instantiate_async
///                  └──► dispatch events ◄──── loop
/// ```
///
/// Plugin state lifecycle:
///
/// ```text
///   install ──────► enable ──────► load ──────► event dispatch ──────► disable ──────► unload ──────► uninstall
/// ```
// # Lock discipline
//
// The global [`PLUGIN_MANAGER`] is protected by an `RwLock` to allow
// concurrent readers. All access MUST follow these rules:
//
// 1. **Release before Wasm/plugin execution**: Any lock (read or write)
//    MUST be released before entering Wasm execution context (e.g.,
//    `call_on_event`, `instantiate_async`). Holding a lock across a
//    Wasm call can cause deadlock when the plugin calls back into the host
//    (e.g., `unregister_event_listener` through [`Self::unbind_event_from_wasm`]).
//
// 2. **Async functions should not use `&self`/`&mut self`**: Holding a
//    borrow across `.await` causes cross-await borrow issues (lifetime
//    violations). Use associated functions (no `&self`) and explicitly
//    acquire/release locks within the function body.
//
// 3. **Keep lock scopes narrow**: Always use a block `{ ... }` to bound
//    the lock scope. Never hold a lock longer than needed.
impl PluginManager {
    /// Acquire a read lock on `PLUGIN_MANAGER`.
    ///
    /// The guard MUST be dropped before entering any Wasm or plugin
    /// execution context (see lock discipline above).
    #[inline]
    pub fn get_plugin_manager() -> RwLockReadGuard<'static, PluginManager> {
        PLUGIN_MANAGER.read().unwrap()
    }

    /// Acquire a write lock on `PLUGIN_MANAGER`.
    ///
    /// The guard MUST be dropped before entering any Wasm or plugin
    /// execution context or before any `.await` point.
    #[inline]
    pub fn get_plugin_manager_mut() -> RwLockWriteGuard<'static, PluginManager> {
        PLUGIN_MANAGER.write().unwrap()
    }

    /// Initialize the plugin manager.
    ///
    /// Loads persisted plugin state, initializes all enabled plugins
    /// (dependency resolution + wasm compilation), then starts the
    /// runtime event loop for each wasm component.
    pub async fn init() {
        {
            let plugin_manager_stat = PluginManager::load_plugin_manager_stat().await.unwrap();

            let mut plugin_manager = PluginManager::get_plugin_manager_mut();

            plugin_manager.stat = plugin_manager_stat;

            plugin_manager._init();

            log::info!("initialized plugin manager");
        } // Leaving the life cycle of `plugin_manager`

        // The lock of `plugin_manager` has been released
        // PluginManager::lifecycle_manager();
        // let local_set = LocalSet::new();
        // let fut = local_set
        //   .run_until(async {
        // tokio::task::spawn_local(PluginManager::lifecycle_manager()).await
        // });
        PluginManager::lifecycle_manager().await
    }

    /// Internal initialization: load all enabled plugins.
    fn _init(&mut self) {
        let enabled_plugins: HashSet<PluginName> =
            HashSet::from_iter(self.stat.enabled_plugins.iter().map(|x| x.name.clone()));

        let plugin_path = &Configuration::get_global_configuration().cosmox.plugin.path;

        let external_lz_plugins = self.plugin_loader.pre_load_external_plugins(plugin_path);
        let builtin_lz_plugins = self.plugin_loader.pre_load_builtin_plugins();

        let mut register_names = |new: Vec<LazyLoadPlugin>| {
            let start = self.plugin_loader.lazy_load_plugins_list.len();
            for (i, lz) in new.iter().enumerate() {
                let name = match lz {
                    LazyLoadPlugin::BuiltinPlugin { name, .. }
                    | LazyLoadPlugin::ExternalPlugin { name, .. }
                    | LazyLoadPlugin::InvalidPlugin { name, .. } => name.clone(),
                };
                self.plugin_loader
                    .lazy_load_plugins_names
                    .insert(name, start + i);
            }
            self.plugin_loader.lazy_load_plugins_list.extend(new);
        };

        register_names(external_lz_plugins);
        register_names(builtin_lz_plugins);

        let _plugins = self.plugin_loader.load_enabled_plugins(enabled_plugins);
    }

    /// Initialize a wasmtime Store for a given wasm component.
    ///
    /// Sets up WASI context, HTTP context, resource table, and plugin
    /// data (binding events, IDs).
    ///
    /// # Arguments
    /// * `engine` - Wasmtime engine.
    /// * `wasm_component` - The compiled wasm component to create a store for.
    ///
    /// # Returns
    /// * `Store<ComponentRunStates>` - Initialized store with full runtime context.
    fn init_wasm_store_fn(
        engine: Engine,
        wasm_component: WasmComponent,
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

    /// Runs the runtime event loop for each wasm component.
    ///
    /// # Lock discipline
    /// The PM read lock is acquired only to read engine and wasm component list,
    /// then released before any Wasm instantiation or execution.
    ///
    /// ```text
    ///   get wasm_list from plugin_loader
    ///         │
    ///         │ for each wasm_component
    ///         ▼
    ///   SHARE cache ──► pop by wasm_id
    ///         │
    ///         ├── hit ────► use cached Store
    ///         └── miss ───► init_wasm_store_fn
    ///                              │
    ///                              └──► create Store (WasiCtx + ResourceTable)
    ///         │
    ///         ▼
    ///   instantiate_async ──────► create PluginHostWorld instance
    ///         │
    ///         ▼
    ///   plugin_wasm_lifecycle ──────► dispatch events to wasm
    ///         │
    ///         ▼
    ///   SHARE cache ──► put store back ◄──── loop
    /// ```
    async fn lifecycle_manager() {
        let (engine, wasm_list) = {
            let plugin_manager = PluginManager::get_plugin_manager();
            let wasm_list = plugin_manager.plugin_loader.get_wasm_compoents();
            let engine = plugin_manager.engine.clone();
            (engine, wasm_list)
        };

        log::debug!("engine config = {:#?}", engine.config());

        for wasm_component in wasm_list.iter() {
            let wasm_id = wasm_component.id;
            log::debug!(
                "start lifetime manager for wasm ID:{wasm_id}, wasm_component:{wasm_component:#?}"
            );

            let mut store = {
                let mut cache = SHARE.lock().unwrap();
                cache.pop(&wasm_id).unwrap_or_else(|| {
                    log::trace!("[Thread '{}' (ID: {:?})] Lazily initializing Store for ServiceKey {}. Cache size: {}",
                        std::thread::current().name().unwrap_or("unnamed"),
                        std::thread::current().id(),
                        wasm_id,
                        cache.len());
                    PluginManager::init_wasm_store_fn(engine.clone(), wasm_component.clone())
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
                cache.put(wasm_id, store);
            }
        }
    }

    async fn extract_plugin<P: AsRef<Path>>(archive_path: &P) -> Result<PathBuf, PluginError> {
        let dst_path = PathBuf::from(
            Configuration::get_global_configuration()
                .cosmox
                .plugin
                .path
                .as_str(),
        );

        cosmox_plugin_packager::unpack(archive_path, dst_path)
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| PluginError::InvalidPluginPackage(err.to_string()))
    }

    pub async fn install_plugin_from_url(url: Url) -> Result<(), PluginError> {
        let resp = reqwest::get(url.clone())
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| PluginError::NetworkTransportError {
                url: url.clone(),
                details: err.to_string(),
            })?;

        if resp.status().is_success() {
            PluginManager::install_plugin_from_stream(resp.bytes_stream()).await
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

        let _plugin_path = PluginManager::extract_plugin(&tmp_path).await?;

        log::info!("Unpack plugin archive successfully.");

        let _plugin_manager = PluginManager::get_plugin_manager_mut();

        // plugin_manager.plugin_loader.load_plugin(lz_plugin)
        // let plugin = plugin_loader::load(plugin_path)?;
        // let id = usize::from(plugin.id());
        // PluginManager::get_plugin_manager_mut().plugins[id] = Some(plugin);

        Ok(())
    }

    /// Uninstall a plugin (placeholder).
    pub fn uninstall() {}

    /// Check plugin health (placeholder).
    pub fn check() {}

    /// Enable a plugin by name.
    ///
    /// Loads the plugin and appends it to the enabled plugin state.
    /// No-op if the plugin is already enabled.
    ///
    /// # Arguments
    /// * `plugin` - Name of the plugin to enable.
    ///
    /// # Returns
    /// * `Ok(())` - Plugin enabled successfully.
    /// * `Err(PluginError)` - Load failure or state persistence error.
    #[inline]
    pub async fn enable(plugin: PluginName) -> Result<(), PluginError> {
        let loaded_plugin;
        {
            let mut plugin_manager = PluginManager::get_plugin_manager_mut();

            if plugin_manager
                .stat
                .enabled_plugins
                .iter()
                .any(|x| x.name == plugin)
            {
                log::warn!("Plugin {plugin} has been enabled.");
                return Ok(());
            }

            loaded_plugin = plugin_manager
                .plugin_loader
                .load_enabled_plugin(plugin.clone())?;

            plugin_manager
                .stat
                .enabled_plugins
                .push(PluginState { name: plugin });
        }

        loaded_plugin.enable();
        Self::store_plugin_manager_stat().await?;

        Ok(())
    }

    /// Disable and unload a plugin by name.
    ///
    /// Frees all wasm IDs, unbinds events, releases slot IDs, and
    /// removes the plugin from the enabled state.
    ///
    /// # Arguments
    /// * `plugin` - Name of the plugin to disable.
    ///
    /// # Returns
    /// * `Ok(())` - Plugin disabled successfully.
    /// * `Err(PluginError)` - Plugin not found.
    #[inline]
    pub async fn disable(plugin: PluginName) -> Result<(), PluginError> {
        {
            let mut plugin_manager = PluginManager::get_plugin_manager_mut();

            if plugin_manager
                .plugin_loader
                .unload_plugin(&plugin)
                .is_none()
            {
                log::error!("Failed disable plugin {plugin}: Not found");
                return Err(PluginError::NotFound(plugin.to_string()));
            }

            plugin_manager
                .stat
                .enabled_plugins
                .retain(|x| x.name != plugin);

            log::info!("Disabled plugin {plugin}");
        }

        Self::store_plugin_manager_stat().await?;

        Ok(())
    }

    /// Load persisted plugin manager state from disk.
    ///
    /// Reads `plugin_state.json` from the configured state path and
    /// deserializes it into `PluginManagerState`.
    /// Return the `PluginManagerState::default()` value if the `plugin_state.json` file is missing
    ///
    /// # Returns
    /// * `Ok(PluginManagerState)` - The loaded state.
    /// * `Err(PluginError)` - I/O or deserialization failure.
    pub async fn load_plugin_manager_stat() -> Result<PluginManagerState, PluginError> {
        let state_path =
            PathBuf::from(&Configuration::get_global_configuration().cosmox.state.path)
                .join("plugin_state.json");

        if fs::exists(&state_path).is_ok_and(|x| !x) {
            return Ok(PluginManagerState::default());
        }

        let mut file = File::open(state_path)
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| PluginError::FileSystemError(err.to_string()))?;
        let mut buf = Vec::with_capacity(512);

        let _ = file
            .read_to_end(&mut buf)
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| PluginError::FileSystemError(err.to_string()))?;

        serde_json::from_slice(buf.as_slice())
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| PluginError::InternalError(err.to_string()))
    }

    /// Persist plugin manager state to disk.
    ///
    /// Serializes the current `PluginManagerState` to `plugin_state.json`
    /// in the configured state directory.
    ///
    /// # Returns
    /// * `Ok(())` - State saved successfully.
    /// * `Err(PluginError)` - I/O or serialization failure.
    pub async fn store_plugin_manager_stat() -> Result<(), PluginError> {
        let stat = PluginManager::get_plugin_manager().stat.clone();
        let state_path =
            PathBuf::from(&Configuration::get_global_configuration().cosmox.state.path);

        fs::create_dir_all(&state_path)
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| PluginError::FileSystemError(err.to_string()))?;

        let mut file = File::create(state_path.join("plugin_state.json"))
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| PluginError::FileSystemError(err.to_string()))?;

        file.write(serde_json::to_vec(&stat).unwrap().as_slice())
            .await
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| PluginError::InternalError(err.to_string()))?;
        Ok(())
    }

    /// Query available UI extensions (placeholder).
    pub fn query_ui_extensions() {}

    /// Query all plugins and return their management info.
    ///
    /// Merges the lazy-load discovery list (all plugins found on disk/builtin)
    /// with the currently enabled (loaded) set to produce a complete picture.
    pub fn query_plugins() -> Vec<PluginQueryItem> {
        let pm = Self::get_plugin_manager();
        let mut items = Vec::with_capacity(pm.plugin_loader.lazy_load_plugins_list.len());

        for lz in &pm.plugin_loader.lazy_load_plugins_list {
            match lz {
                LazyLoadPlugin::BuiltinPlugin {
                    name,
                    version,
                    description,
                } => items.push(PluginQueryItem {
                    name: name.to_string(),
                    version: version.to_string(),
                    description: description.clone(),
                    author: String::new(),
                    email: String::new(),
                    enabled: pm.plugin_loader.plugin_names.contains_key(name),
                    plugin_type: "builtin".to_string(),
                    error: None,
                }),
                LazyLoadPlugin::ExternalPlugin {
                    name,
                    version,
                    description,
                    author,
                    email,
                    ..
                } => items.push(PluginQueryItem {
                    name: name.to_string(),
                    version: version.to_string(),
                    description: description.clone(),
                    author: author.clone(),
                    email: email.clone(),
                    enabled: pm.plugin_loader.plugin_names.contains_key(name),
                    plugin_type: "external".to_string(),
                    error: None,
                }),
                LazyLoadPlugin::InvalidPlugin { name, error } => items.push(PluginQueryItem {
                    name: name.to_string(),
                    version: String::new(),
                    description: String::new(),
                    author: String::new(),
                    email: String::new(),
                    enabled: false,
                    plugin_type: "invalid".to_string(),
                    error: Some(error.to_string()),
                }),
            }
        }

        items
    }

    /// Dispatch events to registered handlers (placeholder).
    pub fn event_dispatcher() {}

    /// Notify all wasm components registered for a given event.
    ///
    /// Encodes the event payload, resolves the registered components,
    /// initializes stores, and dispatches the event to each component
    /// sequentially.
    ///
    /// # Arguments
    /// * `event` - The event to dispatch.
    /// * `event_context_provider` - Closure that builds an `EventContext` from each store.
    ///
    /// # Returns
    /// * `Ok(())` - All components notified successfully.
    /// * `Err(anyhow::Error)` - Encoding or dispatch failure.
    pub async fn notify_all<F>(
        event: Arc<cosmox_api::event::Event>,
        event_context_provider: F,
    ) -> Result<()>
    where
        F: Fn(&mut Store<ComponentRunStates>) -> bindings_context::EventContext,
    {
        log::debug!("notify all message by event{event:?}");
        let current_task_name = "notify event";

        let (components_for_current_event, engine) = {
            let plugin_manager = PluginManager::get_plugin_manager();
            let engine = plugin_manager.engine.clone();
            let components_for_current_event = match plugin_manager
                .plugin_loader
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

    /// Register a wasm component as a listener for an event.
    ///
    /// When the event fires, the wasm component's handler will be invoked.
    ///
    /// # Arguments
    /// * `event` - The event key to listen for.
    /// * `wasm_id` - ID of the wasm component to register.
    ///
    /// # Returns
    /// * `Ok(())` - Registration successful.
    /// * `Err(anyhow::Error)` - Wasm component not found.
    #[inline]
    pub fn bind_event_for_wasm(
        event: cosmox_api::event::EventKey,
        wasm_id: PluginWasmId,
    ) -> Result<()> {
        let mut plugin_manager = PluginManager::get_plugin_manager_mut();
        if let Some(wasm_component) = plugin_manager.plugin_loader.get_wasm_compoent(wasm_id) {
            let wasm_component = wasm_component.clone();

            plugin_manager
                .plugin_loader
                .event_map_to_wasm_components
                .entry(event)
                .or_default()
                .push(wasm_component);
            Ok(())
        } else {
            Err(anyhow!("Not found wasm component by id {wasm_id}"))
        }
    }

    /// Remove a wasm component from an event's listener list.
    ///
    /// After unbinding, the wasm component will no longer receive
    /// notifications for the event.
    ///
    /// # Arguments
    /// * `event` - The event key to unbind from.
    /// * `wasm_id` - ID of the wasm component to remove.
    ///
    /// # Returns
    /// * `Ok(())` - Unbind successful (no-op if not found).
    /// * `Err(anyhow::Error)` - Plugin manager lock failure.
    #[inline]
    pub fn unbind_event_from_wasm(
        event: cosmox_api::event::EventKey,
        wasm_id: PluginWasmId,
    ) -> Result<()> {
        let mut plugin_manager = PluginManager::get_plugin_manager_mut();
        plugin_manager
            .plugin_loader
            .event_map_to_wasm_components
            .entry(event)
            .and_modify(|x| {
                x.retain(|wasm_component| wasm_component.id != wasm_id);
            });
        Ok(())
    }

    /// Add media types to cosmox.
    ///
    /// # Arguments
    /// * `media_types` - A list of media type strings to register.
    ///
    /// # Returns
    /// * `Ok(())` - Media types added successfully.
    /// * `Err(bindings_cosmox_types::MediaTypeError)` - Validation or persistence failure.
    pub async fn push_media_types(
        media_types: Vec<String>,
    ) -> Result<(), bindings_cosmox_types::MediaTypeError> {
        // Validate and update in-memory cache (holding lock briefly)
        {
            let mut plugin_manager = PluginManager::get_plugin_manager_mut();
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

    /// Get the wasmtime runtime engine.
    ///
    /// # Example
    /// ```rust
    /// let engine = PluginManager::get_wasm_engine();
    /// ```
    #[inline]
    pub fn get_wasm_engine() -> Engine {
        PluginManager::get_plugin_manager().engine.clone()
    }

    /// Get the set of MIME types supported by installed plugins.
    ///
    /// # Returns
    /// * `Arc<HashSet<String>>` - Set of supported media type strings.
    pub fn get_supported_media_types() -> Arc<HashSet<String>> {
        Arc::new(
            PluginManager::get_plugin_manager()
                .supported_media_types
                .clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_manager_entity() {
        println!("{:#?}", PluginManager::get_plugin_manager());
    }
}
