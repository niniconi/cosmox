use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::{fs, path::Path};

use anyhow::Result;
use cosmox_configuration::Configuration;
use tracing::{Level, span};
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::*;
// use wasmtime_wasi::p2::bindings::sync::Command;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use bindings::cosmox::plugin::context as bindings_context;
use bindings::cosmox::plugin::cosmox_api as bindings_cosmox_api;
use bindings::cosmox::plugin::cosmox_types as bindings_cosmox_types;
use cosmox_api::{self, event::Event};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

use crate::plugin_manager::PluginManager;
use crate::types::{
    About, Dependency, LazyLoadPlugin, Plugin, PluginId, PluginName, PluginRaw, PluginWasmId,
    PluginWasmName, Version, VersionRequirement, WasmComponent, WasmComponentRaw, WasmUiExtension,
};
use cosmox_agent::ai::call_llm;

pub mod bindings {
    pub use super::super::context::event::{MetadataContext, PathMappingContext, TagContext};
    use wasmtime::component::bindgen;

    bindgen!({
        path: "../cosmox-api/wit",
        world: "plugin-host-world",
        exports: {default: async | trappable},
        with: {
            "cosmox:plugin/context.metadata-handle": MetadataContext,
            "cosmox:plugin/context.path-mapping-handle": PathMappingContext,
            "cosmox:plugin/context.tag-handle": TagContext,
        },
        imports: { "cosmox:plugin/cosmox-api.ai" : async, "cosmox:plugin/context":  trappable }
    });
}

#[derive(Debug, thiserror::Error)]
pub enum PluginLoadError {
    #[error("Plugin not found at path: {0}")]
    NotFound(String),

    #[error("Dependency error: {0}")]
    DependencyError(#[from] PluginDependencyError),

    #[error("Wasm component '{0}' failed to initialize: {1}")]
    WasmInstantiationError(String, String),

    #[error("IO error while loading plugin: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Pre-load plugin error: {0}")]
    PreLoadError(#[from] PluginPreloadError),

    #[error("Parallel load failed: {0} plugins failed to initialize")]
    ParallelBatchError(usize),

    #[error(
        "Incompatible engine version: Plugin '{id}' requires core version '{required}', but current version is '{current}'."
    )]
    EngineIncompatible {
        id: String,
        required: String,
        current: String,
    },

    #[error(
        "Parallel batch load failed: {0} plugins in the dependency group failed to initialize."
    )]
    BatchFailure(usize),
}

#[derive(Debug, thiserror::Error)]
#[error("{error} (plugin id is {id})")]
pub struct PluginDependencyError {
    id: PluginId,
    error: PluginDependencyErrorInner,
}

#[derive(Debug, thiserror::Error)]
enum PluginDependencyErrorInner {
    #[error("Dependency cycle detected involving plugin {0}")]
    Circular(String),

    #[error("Missing required dependency: {requirement} (required by {name})")]
    Missing {
        name: PluginName,
        requirement: String,
    },

    #[error(
        "Conflict detected: Plugin '{name}' conflicts with '{conflicting_with}'. Reason is {reason}"
    )]
    Conflict {
        name: PluginName,
        conflicting_with: String,
        reason: String,
    },
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum PluginPreloadError {
    #[error("Plugin metadata is invalid {0}")]
    InvalidManifest(String),

    #[error("Missing manifest file (about.toml/json).")]
    MissingManifest,

    #[error("Plugin ID allocation failed, the maximum capacity of {0} has been reached")]
    PluginIdCapacityReached(usize),

    #[error("Plugin Wasm ID allocation failed, the maximum capacity of {0} has been reached")]
    PluginWasmIdCapacityReached(usize),
}

const PLUGIN_ID_BITMAP_SIZE: usize = (u16::MAX >> 6) as usize;
const PLUGIN_LIMIT: usize = u16::MAX as usize;
const PLUGIN_WASM_ID_BITMAP_SIZE: usize = (u16::MAX >> 6) as usize;
const PLUGIN_WASM_LIMIT: usize = u16::MAX as usize;

#[derive(Debug)]
pub struct PluginLoader {
    engine: Engine,
    pub plugin_names: HashMap<PluginName, PluginId>,

    plugins: Vec<Option<Plugin>>,
    plugin_ids_bitmap: [u64; PLUGIN_ID_BITMAP_SIZE],

    wasm_components: Vec<Option<WasmComponent>>,
    wasm_components_ids_bitmap: [u64; PLUGIN_WASM_ID_BITMAP_SIZE],

    pub event_map_to_wasm_components: HashMap<cosmox_api::event::EventKey, Vec<WasmComponent>>,

    pub plugin_enable_count: usize,

    /// lazy load plugins contains enable or disable state plugin.
    /// external and builtin plugin will be preload to `LazyLoadPlugin` before load enabled plugins.
    pub lazy_load_plugins_list: Vec<LazyLoadPlugin>,
    pub lazy_load_plugins_names: HashMap<PluginName, usize>,

    /// Stores indices into `lazy_load_plugins_list` for dependency resolution.
    /// Uses indices instead of references to avoid self-referential borrow conflicts.
    pub enabled_ref_lz_plugins: HashMap<PluginId, usize>,
}

pub struct ComponentRunStates {
    pub wasi_ctx: WasiCtx,
    pub wasi_http_ctx: WasiHttpCtx,
    pub resource_table: ResourceTable,
    pub plugin_data: CosmoxPluginData,
}

impl Debug for ComponentRunStates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ComponentRunStates")
    }
}

impl WasiView for ComponentRunStates {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resource_table,
        }
    }
}

impl WasiHttpView for ComponentRunStates {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.wasi_http_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

pub struct CosmoxPluginData {
    pub bind_events: Mutex<Vec<Arc<cosmox_api::event::Event>>>,
    pub plugin_id: PluginId,
    pub wasm_id: PluginWasmId,
    pub name: PluginWasmName,
}

impl bindings_context::Host for ComponentRunStates {}

impl bindings_cosmox_api::Host for CosmoxPluginData {
    fn ai(
        &mut self,
        prompt: String,
    ) -> impl Future<Output = Result<String, bindings_cosmox_types::AiApiError>> {
        async fn warp(prompt: String) -> Result<String, bindings_cosmox_types::AiApiError> {
            call_llm(prompt.as_str())
                .await
                .map_err(|err| bindings_cosmox_types::AiApiError::InternalError(err.to_string()))
        }

        warp(prompt)
    }

    fn log(&mut self, log: bindings_cosmox_types::LogLevel, message: String) {
        match log {
            bindings_cosmox_types::LogLevel::Info => {
                let span = span!(Level::INFO,"cosmox::plugin", plugin = %self.name);
                let _enter = span.enter();
                log::info!("{message}")
            }
            bindings_cosmox_types::LogLevel::Trace => {
                let span = span!(Level::TRACE, "cosmox::plugin", plugin = %self.name);
                let _enter = span.enter();
                log::trace!("{message}")
            }
            bindings_cosmox_types::LogLevel::Debug => {
                let span = span!(Level::DEBUG, "cosmox::plugin", plugin = %self.name);
                let _enter = span.enter();
                log::debug!("{message}")
            }
            bindings_cosmox_types::LogLevel::Warn => {
                let span = span!(Level::WARN, "cosmox::plugin", plugin = %self.name);
                let _enter = span.enter();
                log::warn!("{message}")
            }
            bindings_cosmox_types::LogLevel::Error => {
                let span = span!(Level::ERROR, "cosmox::plugin", plugin = %self.name);
                let _enter = span.enter();
                log::error!("{message}")
            }
            bindings_cosmox_types::LogLevel::Fatal => {
                let span = span!(Level::ERROR, "cosmox::plugin", plugin = %self.name);
                let _enter = span.enter();
                log::error!("{message}")
            }
        }
    }

    fn register_event_listener(
        &mut self,
        event: Vec<u8>,
    ) -> Result<(), bindings_cosmox_types::ListenerRegistrationError> {
        match Event::decode(event) {
            Ok(event) => {
                let event = Arc::new(event);

                if let Err(err) = PluginManager::bind_event_for_wasm(event.into_key(), self.wasm_id)
                {
                    log::error!("Failed to bind event for wasm {}: {err}", self.wasm_id);
                    return Err(bindings_cosmox_api::ListenerRegistrationError::Unknown);
                }

                log::info!("registerd {event:?} for wasm {}", self.wasm_id);
                self.bind_events.lock().unwrap().push(event.clone()); // TODO Optimize this
                Ok(())
            }
            Err(err) => {
                log::error!("{err}");
                Err(
                    bindings_cosmox_api::ListenerRegistrationError::EventPayloadDecodeError(
                        err.to_string(),
                    ),
                )
            }
        }
    }

    fn query_host_context(&mut self, query_key: String, context_id: String) -> Option<Vec<u8>> {
        log::warn!(
            "query_host_context called with key={query_key}, context_id={context_id} but not implemented"
        );
        None
    }

    fn unregister_event_listener(
        &mut self,
        event: Vec<u8>,
    ) -> std::result::Result<(), bindings_cosmox_api::ListenerRegistrationError> {
        match Event::decode(event) {
            Ok(event) => {
                let event = Arc::new(event);

                if let Err(err) =
                    PluginManager::unbind_event_from_wasm(event.into_key(), self.wasm_id)
                {
                    log::error!("Failed to unbind event for wasm {}: {err}", self.wasm_id);
                    return Err(bindings_cosmox_api::ListenerRegistrationError::Unknown);
                }
                Ok(())
            }
            Err(err) => {
                log::error!("{err}");
                Err(
                    bindings_cosmox_api::ListenerRegistrationError::EventPayloadDecodeError(
                        err.to_string(),
                    ),
                )
            }
        }
    }

    fn get_supported_media_types(&mut self) -> Vec<String> {
        PluginManager::get_supported_media_types()
            .iter()
            .cloned()
            .collect()
    }
}

impl PluginLoader {
    pub fn new(engine: Engine) -> Self {
        PluginLoader {
            engine,
            plugin_names: HashMap::new(),
            plugins: vec![None::<Plugin>; PLUGIN_LIMIT],
            plugin_ids_bitmap: [0u64; PLUGIN_ID_BITMAP_SIZE],
            wasm_components: vec![None::<WasmComponent>; PLUGIN_WASM_LIMIT],
            wasm_components_ids_bitmap: [0u64; PLUGIN_WASM_ID_BITMAP_SIZE],
            event_map_to_wasm_components: HashMap::new(),
            plugin_enable_count: 0,
            lazy_load_plugins_list: Vec::with_capacity(128),
            lazy_load_plugins_names: HashMap::new(),
            enabled_ref_lz_plugins: HashMap::new(),
        }
    }
}

impl Clone for PluginLoader {
    fn clone(&self) -> Self {
        let mut plugin_ids_bitmap = [0u64; PLUGIN_ID_BITMAP_SIZE];
        plugin_ids_bitmap.copy_from_slice(&self.plugin_ids_bitmap);
        let mut wasm_components_ids_bitmap = [0u64; PLUGIN_WASM_ID_BITMAP_SIZE];
        wasm_components_ids_bitmap.copy_from_slice(&self.wasm_components_ids_bitmap);

        Self {
            engine: self.engine.clone(),
            plugin_names: self.plugin_names.clone(),
            plugins: self.plugins.clone(),
            plugin_ids_bitmap,
            wasm_components: self.wasm_components.clone(),
            wasm_components_ids_bitmap,
            event_map_to_wasm_components: self.event_map_to_wasm_components.clone(),
            plugin_enable_count: self.plugin_enable_count,
            lazy_load_plugins_list: self.lazy_load_plugins_list.clone(),
            lazy_load_plugins_names: self.lazy_load_plugins_names.clone(),
            enabled_ref_lz_plugins: self.enabled_ref_lz_plugins.clone(),
        }
    }
}

impl PluginLoader {
    fn alloc_plugin_id(&mut self) -> Result<PluginId, PluginLoadError> {
        for idx in 0..PLUGIN_ID_BITMAP_SIZE {
            let offset = self.plugin_ids_bitmap[idx].trailing_ones();
            if offset < 64 {
                self.plugin_ids_bitmap[idx] |= 1 << offset;
                return Ok(PluginId::new(idx as u64 * 64 + offset as u64));
            }
        }
        Err(PluginPreloadError::PluginIdCapacityReached(PLUGIN_LIMIT).into())
    }

    fn free_plugin_id(&mut self, plugin_id: PluginId) {
        let idx = (*plugin_id >> 6) as usize;
        self.plugin_ids_bitmap[idx] &= !(1u64 << (*plugin_id & 0x3f));
    }

    fn alloc_plugin_wasm_id(&mut self) -> Result<PluginWasmId, PluginLoadError> {
        for idx in 0..PLUGIN_WASM_ID_BITMAP_SIZE {
            let offset = self.wasm_components_ids_bitmap[idx].trailing_ones();
            if offset < 64 {
                self.wasm_components_ids_bitmap[idx] |= 1 << offset;
                return Ok(PluginWasmId::new(idx as u64 * 64 + offset as u64));
            }
        }
        Err(PluginPreloadError::PluginWasmIdCapacityReached(PLUGIN_WASM_LIMIT).into())
    }

    fn free_plugin_wasm_id(&mut self, plugin_wasm_id: PluginWasmId) {
        let idx = (*plugin_wasm_id >> 6) as usize;
        self.wasm_components_ids_bitmap[idx] &= !(1u64 << (*plugin_wasm_id & 0x3f));
    }

    fn insert_plugin_name(&mut self, plugin_name: PluginName, plugin_id: PluginId) {
        self.plugin_names.insert(plugin_name, plugin_id);
    }

    #[inline]
    fn register_wasm_component(&mut self, wasm_id: PluginWasmId, wasm_component: WasmComponent) {
        self.wasm_components[usize::from(wasm_id)] = Some(wasm_component)
    }

    #[inline]
    fn register_plugin(&mut self, plugin_id: PluginId, plugin: Plugin) {
        self.plugins[usize::from(plugin_id)] = Some(plugin)
    }

    pub fn get_plugins(&self) -> Arc<[Plugin]> {
        self.plugins.iter().filter_map(|x| x.clone()).collect()
    }

    pub fn get_wasm_compoents(&self) -> Arc<[WasmComponent]> {
        self.wasm_components
            .iter()
            .filter_map(|x| x.clone())
            .collect()
    }

    pub fn get_plugin(&self, plugin_id: PluginId) -> Option<Plugin> {
        self.plugins[usize::from(plugin_id)].clone()
    }
    pub fn get_wasm_compoent(&self, plugin_wasm_id: PluginWasmId) -> Option<WasmComponent> {
        self.wasm_components[usize::from(plugin_wasm_id)].clone()
    }

    /// Unload a plugin: frees wasm IDs, unbinds events, releases slot IDs.
    /// Returns `Some(plugin_id)` if the plugin was found and unloaded.
    ///
    /// # Arguments
    /// * `plugin` - Name of the plugin to unload.
    ///
    /// # Returns
    /// * `Some(PluginId)` - The freed plugin ID if found and unloaded.
    /// * `None` - Plugin was not found.
    pub fn unload_plugin(&mut self, plugin: &PluginName) -> Option<PluginId> {
        let plugin_id = self.plugin_names.get(plugin).copied()?;
        let idx = usize::from(plugin_id);

        let wasm_ids: Vec<PluginWasmId> = self.plugins[idx]
            .take()
            .map(|plugin_arc| match plugin_arc.as_ref() {
                PluginRaw::ExternalPlugin {
                    wasm_extensions, ..
                } => wasm_extensions
                    .iter()
                    .flat_map(|m| m.keys())
                    .copied()
                    .collect(),
                PluginRaw::BuiltinPlugin { .. } => vec![],
            })
            .unwrap_or_default();

        for &wasm_id in &wasm_ids {
            self.wasm_components[usize::from(wasm_id)] = None;
            self.free_plugin_wasm_id(wasm_id);
        }

        if !wasm_ids.is_empty() {
            self.event_map_to_wasm_components.retain(|_, components| {
                components.retain(|wc| !wasm_ids.contains(&wc.id));
                !components.is_empty()
            });
        }

        self.plugin_names.remove(plugin);
        self.enabled_ref_lz_plugins.remove(&plugin_id);
        self.plugin_enable_count = self.plugin_enable_count.saturating_sub(1);
        self.free_plugin_id(plugin_id);

        Some(plugin_id)
    }

    /// Load a wasm plugin from disk.
    /// Performs actual wasm JIT compilation; future versions will support
    /// loading from a pre-compiled wasm JIT cache.
    ///
    /// # Arguments
    /// * `plugin_id` - Plugin ID to associate the wasm component with.
    /// * `path` - Filesystem path to the wasm binary.
    /// * `engine` - Wasmtime engine used for compilation.
    ///
    /// # Returns
    /// * `Ok(WasmComponent)` - The compiled wasm component.
    /// * `Err(PluginLoadError)` - Compilation or I/O failure.
    pub fn load_wasm<P>(&mut self, plugin_id: PluginId, path: P) -> Result<WasmComponent>
    where
        P: AsRef<Path>,
    {
        let id = self.alloc_plugin_wasm_id()?;
        let engine = &self.engine;

        let build = move || -> Result<WasmComponent> {
            let mut linker: Linker<ComponentRunStates> = Linker::new(engine);

            // wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
            wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
            wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)?;

            // Create a WASI context and put it in a Store; all instances in the store
            // share this context. `WasiCtxBuilder` provides a number of ways to
            // configure what the target program will have access to.
            // let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();
            // let bind_events = Mutex::new(Vec::with_capacity(32));
            // let resource_table = ResourceTable::new();
            // let state = ComponentRunStates {
            //   wasi_ctx,
            //   resource_table,
            //   plugin_data: CosmoxPluginData {
            //     bind_events,
            //     plugin_id,
            //     wasm_id,
            //     name: wasm_name,
            //   },
            // };

            let component = Component::from_file(engine, &path)?;
            let name = PluginWasmName::new(
                path.as_ref()
                    .file_name()
                    .map(|x| x.to_str().unwrap().to_string())
                    .unwrap_or("".to_string()),
            );

            bindings::cosmox::plugin::cosmox_api::add_to_linker::<_, HasSelf<_>>(
                &mut linker,
                |s| &mut s.plugin_data,
            )?;

            bindings::cosmox::plugin::context::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;

            let linker = Arc::new(linker);
            let path = PathBuf::from(path.as_ref());

            Ok(Arc::new(WasmComponentRaw {
                id,
                name,
                plugin_id,
                path,
                component,
                linker,
            }))
        };

        let result = build();
        if result.is_err() {
            self.free_plugin_wasm_id(id);
        }
        result
    }

    /// Unload a specific wasm component (placeholder).
    #[allow(dead_code)]
    pub fn unload_wasm(&mut self) {}

    /// Analyze ALL plugin dependencies and build the reverse dependency map.
    ///
    /// # Arguments
    /// * `plugins` - Map of plugin IDs to their lazy load descriptors.
    ///
    /// # Returns
    /// * `(reverse_deps, result)` where:
    ///   * `reverse_deps` - Reverse dependency map (plugin → plugins that depend on it).
    ///   * `result` - `Ok(())` or `Err` with dependency/conflict errors.
    pub fn finalize_dependency_all(
        &self,
        plugins: &HashMap<PluginId, &LazyLoadPlugin>,
    ) -> (
        HashMap<PluginId, Vec<PluginId>>,
        Result<(), Vec<PluginDependencyError>>,
    ) {
        let mut errors = vec![];
        let mut reverse_deps: HashMap<PluginId, Vec<PluginId>> = HashMap::new();

        // Name → PluginId index for O(1) lookups during dep/conflict resolution.
        let name_to_id: HashMap<PluginName, PluginId> = plugins
            .iter()
            .filter_map(|(id, plugin)| {
                let name = match plugin {
                    LazyLoadPlugin::ExternalPlugin { name, .. } => name.clone(),
                    LazyLoadPlugin::BuiltinPlugin { name, .. } => name.clone(),
                    _ => return None,
                };
                Some((name, *id))
            })
            .collect();

        for (id, plugin) in plugins {
            let plugin = *plugin;
            // Ensure every plugin gets an entry, even if nothing depends on it
            reverse_deps.entry(*id).or_default();
            let LazyLoadPlugin::ExternalPlugin {
                name,
                dependencies,
                conflicts,
                ..
            } = plugin
            else {
                continue;
            };

            if let Some(deps) = dependencies {
                for dep in deps {
                    let Some(dep_id) = name_to_id.get(&dep.name).copied() else {
                        log::error!("Plugin {name} is missing dependency {}.", dep.name);
                        errors.push(PluginDependencyError {
                            id: *id,
                            error: PluginDependencyErrorInner::Missing {
                                name: name.clone(),
                                requirement: dep.name.to_string(),
                            },
                        });
                        continue;
                    };

                    let Some(lz_plugin) = plugins.get(&dep_id) else {
                        continue;
                    };

                    let dependency_version = match lz_plugin {
                        LazyLoadPlugin::ExternalPlugin { version, .. } => version,
                        LazyLoadPlugin::BuiltinPlugin { version, .. } => version,
                        _ => continue,
                    };

                    let miss_version = match &dep.requirement {
                        VersionRequirement::Exact(version) => (dependency_version != version).then(|| {
                            log::error!("Dependency Error: '{name}' requires '{target}' at exactly {version}, but found {dependency_version}", target = dep.name);
                        }),
                        VersionRequirement::LessEqual(version) => (dependency_version > version).then(|| {
                            log::error!("Dependency Error: '{name}' requires '{target}' <= {version}, but found {dependency_version} (too high)", target = dep.name);
                        }),
                        VersionRequirement::GreaterEqual(version) => (dependency_version < version).then(|| {
                            log::error!("Dependency Error: '{name}' requires '{target}' >= {version}, but found {dependency_version} (too low)", target = dep.name);
                        }),
                        VersionRequirement::Caret(version) => (!dependency_version.matches_caret(version)).then(|| {
                            log::error!("Dependency Error: '{name}' requires '{target}' ^{version}, but found {dependency_version} (incompatible)", target = dep.name);
                        }),
                        VersionRequirement::Tilde(version) => (!dependency_version.matches_tilde(version)).then(|| {
                            log::error!("Dependency Error: '{name}' requires '{target}' ~{version}, but found {dependency_version} (patch range mismatch)", target = dep.name);
                        }),
                        VersionRequirement::Any => continue,
                    };

                    if miss_version.is_some() {
                        errors.push(PluginDependencyError {
                            id: *id,
                            error: PluginDependencyErrorInner::Missing {
                                name: name.clone(),
                                requirement: dep.to_string(),
                            },
                        })
                    }

                    reverse_deps.entry(dep_id).or_default().push(*id);
                }
            }

            if let Some(conflicts) = conflicts {
                for conflict in conflicts {
                    let Some(&conflict_id) = name_to_id.get(&conflict.name) else {
                        continue;
                    };

                    let Some(lz_plugin) = plugins.get(&conflict_id) else {
                        continue;
                    };

                    let conflict_version = match lz_plugin {
                        LazyLoadPlugin::ExternalPlugin { version, .. } => version,
                        LazyLoadPlugin::BuiltinPlugin { version, .. } => version,
                        _ => continue,
                    };

                    let conflict_reasons = match &conflict.requirement {
                    VersionRequirement::Exact(version) => (conflict_version == version).then(|| {
                        format!("Conflict: '{name}' cannot coexist with '{target}' at version {version}, but {conflict_version} is installed", target = conflict.name)
                    }),
                    VersionRequirement::LessEqual(version) => (conflict_version <= version).then(|| {
                        format!("Conflict: '{name}' is incompatible with '{target}' <= {version} (found {conflict_version} in forbidden range)", target = conflict.name)
                    }),
                    VersionRequirement::GreaterEqual(version) => (conflict_version >= version).then(|| {
                        format!("Conflict: '{name}' is incompatible with '{target}' >= {version} (found {conflict_version} in forbidden range)", target = conflict.name)
                    }),
                    VersionRequirement::Caret(version) => (conflict_version.matches_caret(version)).then(|| {
                        format!("Conflict: '{name}' has a Caret conflict with '{target}' ^{version} (found incompatible {conflict_version})", target = conflict.name)
                    }),
                    VersionRequirement::Tilde(version) => (conflict_version.matches_tilde(version)).then(|| {
                        format!("Conflict: '{name}' has a Tilde conflict with '{target}' ~{version} (found incompatible {conflict_version})", target = conflict.name)
                    }),
                    VersionRequirement::Any => {
                        Some(format!("Plugin {name} conflicts with plugin {}.", conflict.name))
                    }
                };

                    if let Some(conflict_reason) = conflict_reasons {
                        errors.push(PluginDependencyError {
                            id: *id,
                            error: PluginDependencyErrorInner::Conflict {
                                name: name.clone(),
                                conflicting_with: conflict.to_string(),
                                reason: conflict_reason,
                            },
                        })
                    }
                }
            }
        }
        if !errors.is_empty() {
            (reverse_deps, Err(errors))
        } else {
            (reverse_deps, Ok(()))
        }
    }

    /// Analyzes plugin dependencies and returns them in executable batches.
    ///
    /// # Arguments
    /// * `reverse_deps` - A map where the key is a Plugin ID and the value is a list of Plugin IDs that depend on it.
    /// * `total_nodes` - The total number of unique plugin IDs to be processed.
    ///
    /// # Returns
    /// * `Ok(Vec<Vec<PluginId>>)` - Successive batches of plugin IDs that can be executed in parallel.
    /// * `Err(Vec<PluginDependencyError>)` - A list of plugins involved in a circular dependency.
    pub fn dependencies_analyzer(
        reverse_deps: HashMap<PluginId, Vec<PluginId>>,
        total_nodes: usize,
    ) -> Result<Vec<Vec<PluginId>>, Vec<PluginDependencyError>> {
        let mut in_degree: HashMap<PluginId, usize> = HashMap::with_capacity(total_nodes);

        for &id in reverse_deps.keys() {
            in_degree.entry(id).or_insert(0);
        }

        for neighbors in reverse_deps.values() {
            for &neighbor in neighbors {
                *in_degree.entry(neighbor).or_insert(0) += 1;
            }
        }

        let mut current_batch: Vec<PluginId> = in_degree
            .iter()
            .filter(|&(_, &count)| count == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut result = Vec::new();
        let mut visited_count = 0;

        while !current_batch.is_empty() {
            visited_count += current_batch.len();
            let mut next_batch = Vec::new();

            for &id in &current_batch {
                if let Some(dependents) = reverse_deps.get(&id) {
                    for &dep_id in dependents {
                        if let Some(degree) = in_degree.get_mut(&dep_id) {
                            *degree -= 1;
                            if *degree == 0 {
                                next_batch.push(dep_id);
                            }
                        }
                    }
                }
            }

            result.push(current_batch);
            current_batch = next_batch;
        }

        if visited_count != total_nodes {
            let missing: Vec<PluginDependencyError> = in_degree
                .iter()
                .filter(|&(_, &count)| count > 0)
                .map(|(id, _)| PluginDependencyError {
                    id: *id,
                    error: PluginDependencyErrorInner::Circular(id.to_string()),
                })
                .collect();
            return Err(missing);
        }

        Ok(result)
    }

    /// Load a single enabled plugin by name.
    ///
    /// Get index of `LazyLoadPlugin` from `lazy_load_plugins_names` for the plugin, allocates a PluginId,
    /// and dispatches to either `load_external_plugin` or `load_builtin_plugin`.
    /// preload the plugin if the `plugin_name` is not contained in `lazy_load_plugins_names`.
    ///
    /// # Arguments
    /// * `plugin_name` - Name of the plugin to load.
    ///
    /// # Returns
    /// * `Ok(Plugin)` - The loaded plugin.
    /// * `Err(PluginLoadError)` - Plugin not found or load failure.
    pub fn load_enabled_plugin(
        &mut self,
        plugin_name: PluginName,
    ) -> Result<Plugin, PluginLoadError> {
        let idx = match self.lazy_load_plugins_names.get(&plugin_name) {
            Some(idx) => *idx,
            None => {
                let plugin_path =
                    PathBuf::from(&Configuration::get_global_configuration().cosmox.plugin.path)
                        .join(plugin_name.to_string());
                let lz_plugin = match self.pre_load_plugin_from_path(plugin_path) {
                    Ok(lz_plugin) => lz_plugin,
                    Err(err) => {
                        log::error!("Failed enable plugin: {err}");
                        return Err(err);
                    }
                };
                self.lazy_load_plugins_list.push(lz_plugin);
                let idx = self.lazy_load_plugins_list.len().saturating_sub(1);
                self.lazy_load_plugins_names
                    .insert(plugin_name.clone(), idx);
                idx
            }
        };

        let id = self.alloc_plugin_id()?;
        let mut dep_map: HashMap<PluginId, &LazyLoadPlugin> =
            HashMap::with_capacity(self.enabled_ref_lz_plugins.len() + 1);
        for (pid, &stored_idx) in &self.enabled_ref_lz_plugins {
            dep_map.insert(*pid, &self.lazy_load_plugins_list[stored_idx]);
        }
        dep_map.insert(id, &self.lazy_load_plugins_list[idx]);

        let total_nodes = dep_map.len();
        let (reverse_deps, errors) = self.finalize_dependency_all(&dep_map);

        if let Err(errors) = errors {
            log::warn!("finalize dependency errors, count = {}", errors.len());
            self.free_plugin_id(id);
            return Err(PluginLoadError::DependencyError(
                errors.into_iter().next().unwrap(),
            ));
        }

        if let Err(errors) = PluginLoader::dependencies_analyzer(reverse_deps, total_nodes) {
            log::warn!("dependency analyzer errors, count = {}", errors.len());
            self.free_plugin_id(id);
            return Err(PluginLoadError::DependencyError(
                errors.into_iter().next().unwrap(),
            ));
        }

        log::info!("plugin {plugin_name} dependency check successful");

        let lz_plugin = self.lazy_load_plugins_list[idx].clone();
        let plugin = if matches!(lz_plugin, LazyLoadPlugin::BuiltinPlugin { .. }) {
            self.load_builtin_plugin(id, lz_plugin)
        } else if matches!(lz_plugin, LazyLoadPlugin::ExternalPlugin { .. }) {
            self.load_external_plugin(id, lz_plugin)
        } else {
            unreachable!()
        }?;

        self.enabled_ref_lz_plugins.insert(id, idx);
        self.register_plugin(id, plugin.clone());
        self.plugin_enable_count += 1;
        Ok(plugin)
    }

    /// Loads all enabled plugins with dependency-aware batch scheduling.
    ///
    /// ```text
    ///   discover plugins
    ///         │
    ///         │ filter lazy_load_plugin's index by enabled_plugins
    ///         │ preload the plugin if the plugin_name is not contained in lazy_load_plugins_names .
    ///         │
    ///         ▼
    ///   collect need_load_plugins
    ///         │
    ///         │ alloc plugin IDs
    ///         ▼
    ///   finalize_dependency_all
    ///         ├──► resolve dep version requirements
    ///         ├──► detect conflicts
    ///         └──► build reverse_dep map
    ///         │
    ///         ▼
    ///   dependencies_analyzer
    ///         └──► topological sort (Kahn) ──► parallel batches
    ///         │
    ///         ▼
    ///   load_external / load_builtin plugin
    ///         │
    ///         ├──► load_wasm + jit compile ──► external
    ///         ├──► run ──────► builtin
    ///         └──► register_plugin + collect
    ///         │
    ///         ▼
    ///   free leftover IDs ──────► return Vec<Plugin>
    /// ```
    pub fn load_enabled_plugins(
        &mut self,
        plugin_names: HashSet<PluginName>,
    ) -> Result<Vec<Plugin>, PluginLoadError> {
        let plugin_dir_path =
            PathBuf::from(&Configuration::get_global_configuration().cosmox.plugin.path);

        let need_load_plugins_indices = plugin_names
            .iter()
            .map(
                |plugin_name| match self.lazy_load_plugins_names.get(plugin_name) {
                    Some(idx) => Ok(*idx),
                    None => {
                        let plugin_path = plugin_dir_path.join(plugin_name.to_string());
                        let lz_plugin = match self.pre_load_plugin_from_path(plugin_path) {
                            Ok(lz_plugin) => lz_plugin,
                            Err(err) => {
                                log::warn!("Failed enable plugin: {err}");
                                return Err(err);
                            }
                        };
                        self.lazy_load_plugins_list.push(lz_plugin);
                        let idx = self.lazy_load_plugins_list.len().saturating_sub(1);
                        self.lazy_load_plugins_names
                            .insert(plugin_name.clone(), idx);
                        Ok(idx)
                    }
                },
            )
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();

        let matched_count = need_load_plugins_indices.len();

        let mut allocated_ids = Vec::with_capacity(matched_count);
        for _ in 0..matched_count {
            allocated_ids.push(self.alloc_plugin_id()?);
        }

        let mut need_load_plugins: HashMap<PluginId, (usize, &LazyLoadPlugin)> =
            HashMap::with_capacity(matched_count);
        for (id, &idx) in allocated_ids.into_iter().zip(&need_load_plugins_indices) {
            need_load_plugins.insert(id, (idx, &self.lazy_load_plugins_list[idx]));
        }

        let dep_map: HashMap<PluginId, &LazyLoadPlugin> = need_load_plugins
            .iter()
            .map(|(&k, &(_, v))| (k, v))
            .collect();

        let (reverse_deps, errors) = self.finalize_dependency_all(&dep_map);
        let mut total_nodes = matched_count;

        if let Err(errors) = errors {
            log::warn!("finalize dependency errors, count = {}", errors.len());
            total_nodes -= errors.len();
        }

        self.plugin_enable_count += total_nodes;

        let mut need_load_plugins: HashMap<PluginId, (usize, LazyLoadPlugin)> = need_load_plugins
            .into_iter()
            .map(|(id, (idx, plugin))| (id, (idx, plugin.clone())))
            .collect();

        let mut plugins = vec![];
        match Self::dependencies_analyzer(reverse_deps, total_nodes) {
            Ok(load_order) => {
                log::info!("plugins dependency check successful");
                for par_order in load_order {
                    for id in par_order {
                        let Some((_, (idx, lz_plugin))) = need_load_plugins.remove_entry(&id)
                        else {
                            unreachable!();
                        };
                        if matches!(lz_plugin, LazyLoadPlugin::BuiltinPlugin { .. }) {
                            match self.load_builtin_plugin(id, lz_plugin) {
                                Ok(plugin) => {
                                    self.register_plugin(id, plugin.clone());
                                    self.enabled_ref_lz_plugins.insert(id, idx);
                                    plugins.push(plugin)
                                }
                                Err(err) => {
                                    log::error!("Failed to load builtin plugin (id={id}): {err}");
                                    self.free_plugin_id(id);
                                }
                            }
                        } else if matches!(lz_plugin, LazyLoadPlugin::ExternalPlugin { .. }) {
                            match self.load_external_plugin(id, lz_plugin) {
                                Ok(plugin) => {
                                    self.register_plugin(id, plugin.clone());
                                    self.enabled_ref_lz_plugins.insert(id, idx);
                                    plugins.push(plugin)
                                }
                                Err(err) => {
                                    log::error!("Failed to load external plugin (id={id}): {err}");
                                    self.free_plugin_id(id);
                                }
                            }
                        }
                    }
                }
            }
            Err(errors) => {
                for err in errors {
                    let PluginDependencyError { id, error: err } = err;
                    log::error!("{err}");
                    self.free_plugin_id(id);
                }
            }
        }

        // Free IDs excluded from topological sort due to dep/conflict errors.
        for (id, _) in need_load_plugins.drain() {
            log::warn!("Skipping plugin {id}: unresolved dependency errors");
            self.free_plugin_id(id);
        }

        Ok(plugins)
    }

    /// Pre-discover builtin plugins bundled with the application.
    ///
    /// Currently returns a single "core" builtin plugin.
    ///
    /// # Returns
    /// * `Vec<LazyLoadPlugin>` - List of discovered builtin plugin descriptors.
    pub fn pre_load_builtin_plugins(&mut self) -> Vec<LazyLoadPlugin> {
        vec![LazyLoadPlugin::BuiltinPlugin {
            name: PluginName::new("core"),
            version: Version::from(env!("CARGO_PKG_VERSION")),
            description: "".to_string(),
        }]
    }

    /// Pre-discover external plugins from the plugin directory.
    ///
    /// Scans each subdirectory under `path` for plugin manifests (`about.yaml`)
    /// and returns the discovered plugin descriptors.
    ///
    /// # Arguments
    /// * `path` - Root directory to scan for plugins.
    ///
    /// # Returns
    /// * `Vec<LazyLoadPlugin>` - List of discovered plugin descriptors (may include `InvalidPlugin` entries).
    pub fn pre_load_external_plugins<P: AsRef<Path>>(&mut self, path: P) -> Vec<LazyLoadPlugin> {
        let mut lz_plugins = Vec::new();

        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries {
                if let Ok(entry) = entry
                    && let Ok(metadata) = entry.metadata()
                    && metadata.is_dir()
                {
                    match self.pre_load_plugin_from_path(entry.path()) {
                        Ok(plugin) => lz_plugins.push(plugin),
                        Err(err) => {
                            log::error!(
                                "Failed to pre-load plugin from {}: {err}",
                                entry.path().display()
                            );
                        }
                    }
                }
            }
        }

        log::trace!("Loaded external plugins from {}", path.as_ref().display());

        lz_plugins
    }

    /// pre load and validate plugin before real load plugin from path.
    /// Pre-load a single plugin from its directory path.
    ///
    /// Reads the `about.yaml` manifest and returns a `LazyLoadPlugin` descriptor.
    /// If the manifest is missing or malformed, returns `LazyLoadPlugin::InvalidPlugin`.
    ///
    /// # Arguments
    /// * `path` - Directory path containing the plugin's `about.yaml`.
    ///
    /// # Returns
    /// * `Ok(LazyLoadPlugin)` - Plugin descriptor (may be `InvalidPlugin` on failure).
    /// * `Err(PluginLoadError)` - Critical error (e.g., ID capacity reached).
    pub fn pre_load_plugin_from_path<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<LazyLoadPlugin, PluginLoadError> {
        let path = path.as_ref();
        log::trace!("Pre loading plugin from {}", path.display());

        let mut manifest: Option<About> = None;

        if fs::exists(path).is_ok_and(|x| !x) {
            return Err(PluginLoadError::NotFound(path.display().to_string()));
        }

        // find about.yaml and parse.
        if let Ok(dir) = fs::read_dir(path) {
            for entry in dir {
                if let Ok(entry) = entry
                    && let Ok(metadata) = entry.metadata()
                    && metadata.is_file()
                    && entry.file_name() == "about.yaml"
                {
                    // read about.yaml
                    if let Ok(data) = fs::read(path.join("about.yaml")) {
                        let about: About = match serde_yaml::from_slice(data.as_slice()) {
                            Ok(data) => data,
                            Err(err) => {
                                log::error!("Failed to load about.yaml because {err}");

                                let name = path
                                    .components()
                                    .next_back()
                                    .and_then(|c| c.as_os_str().to_str())
                                    .unwrap_or("None");

                                return Ok(LazyLoadPlugin::InvalidPlugin {
                                    name: PluginName::new(name),
                                    error: PluginPreloadError::InvalidManifest(err.to_string()),
                                });
                            }
                        };

                        manifest = Some(about);
                    }
                }
            }
        }

        if let Some(manifest) = manifest {
            log::trace!("Pre loaded external plugin {}", manifest.name);

            let parse_dep: fn(Option<Vec<String>>) -> Option<Vec<Dependency>> = |v| {
                v.map(|x| {
                    x.iter()
                        .map(Dependency::parse)
                        .inspect(|x| {
                            if let Err(err) = x {
                                log::warn!("{err}")
                            }
                        })
                        .flatten()
                        .collect::<Vec<_>>()
                })
            };

            let dependencies = parse_dep(manifest.dependencies);
            let conflicts = parse_dep(manifest.conflicts);
            let name = PluginName::new(manifest.name);
            let version = Version::from(manifest.version);
            let description = manifest.description.unwrap_or("".to_string());
            let author = "".to_string();
            let email = manifest.email.unwrap_or("".to_string());
            let permission = vec![];
            let path = path.to_path_buf();

            Ok(LazyLoadPlugin::ExternalPlugin {
                name,
                author,
                email,
                version,
                description,
                path,
                permission,
                dependencies,
                conflicts,
            })
        } else {
            log::error!("Invalid plugin struct at {}", path.display());

            let name = path
                .components()
                .next_back()
                .and_then(|c| c.as_os_str().to_str())
                .unwrap_or("None");

            Ok(LazyLoadPlugin::InvalidPlugin {
                name: PluginName::new(name),
                error: PluginPreloadError::MissingManifest,
            })
        }
    }

    /// Construct a builtin plugin from its lazy load descriptor.
    ///
    /// # Arguments
    /// * `id` - Pre-allocated PluginId to assign.
    /// * `lz_plugin` - Plugin descriptor (must be `LazyLoadPlugin::BuiltinPlugin`).
    ///
    /// # Returns
    /// * `Ok(Plugin)` - The constructed builtin plugin.
    /// * `Err(PluginLoadError)` - If `lz_plugin` is not a BuiltinPlugin variant.
    pub fn load_builtin_plugin(
        &mut self,
        id: PluginId,
        lz_plugin: LazyLoadPlugin,
    ) -> Result<Plugin, PluginLoadError> {
        let enable = AtomicBool::new(true);
        let (name, version, description) = match lz_plugin {
            LazyLoadPlugin::ExternalPlugin { name, .. } => {
                log::error!("It's not a builtin plugin {name}");
                self.free_plugin_id(id);
                return Err(PluginLoadError::WasmInstantiationError(
                    name.to_string(),
                    "Expected a builtin plugin but got an external plugin".to_string(),
                ));
            }
            LazyLoadPlugin::BuiltinPlugin {
                name,
                version,
                description,
            } => (name, version, description),
            LazyLoadPlugin::InvalidPlugin { error, .. } => {
                log::error!("Loading a invalid plugin: {error}");
                self.free_plugin_id(id);
                return Err(error.into());
            }
        };

        // Free any previously registered pre-load ID for this name
        if let Some(&old_id) = self.plugin_names.get(&name) {
            self.free_plugin_id(old_id);
        }
        self.insert_plugin_name(name.clone(), id);

        let builtin_plugin = Arc::new(PluginRaw::BuiltinPlugin {
            id,
            enable,
            version,
            name,
            description,
        });
        Ok(builtin_plugin)
    }

    /// Load an external plugin from its lazy load descriptor.
    ///
    /// Reads the plugin directory, compiles all wasm extensions, and registers
    /// them as wasm components. Supports `ExternalPlugin`, `BuiltinPlugin`
    /// (returns error), and `InvalidPlugin` (propagates error) variants.
    ///
    /// # Arguments
    /// * `id` - Pre-allocated PluginId to assign.
    /// * `lz_plugin` - Plugin descriptor (typically `LazyLoadPlugin::ExternalPlugin`).
    ///
    /// # Returns
    /// * `Ok(Plugin)` - The fully loaded external plugin with wasm components.
    /// * `Err(PluginLoadError)` - Loading, compilation, or capacity failure.
    pub fn load_external_plugin(
        &mut self,
        id: PluginId,
        lz_plugin: LazyLoadPlugin,
    ) -> Result<Plugin, PluginLoadError> {
        let (name, version, author, email, description, dependencies, conflicts, path) =
            match lz_plugin {
                LazyLoadPlugin::InvalidPlugin { error, .. } => {
                    log::error!("Loading a invalid plugin: {error}");
                    self.free_plugin_id(id);
                    return Err(error.into());
                }
                LazyLoadPlugin::BuiltinPlugin { name, .. } => {
                    log::error!("It's not a external plugin {name}");
                    self.free_plugin_id(id);
                    return Err(PluginLoadError::WasmInstantiationError(
                        name.to_string(),
                        "Not an external plugin".to_string(),
                    ));
                }
                LazyLoadPlugin::ExternalPlugin {
                    name,
                    version,
                    author,
                    email,
                    description,
                    dependencies,
                    conflicts,
                    path,
                    ..
                } => (
                    name,
                    version,
                    author,
                    email,
                    description,
                    dependencies,
                    conflicts,
                    path,
                ),
            };

        log::trace!("Loading plugin {name:?}");

        let mut wasm_extensions: HashMap<PluginWasmId, WasmComponent> = HashMap::with_capacity(16);
        let mut _wasm_ui_extensions: HashMap<PluginWasmId, WasmUiExtension> =
            HashMap::with_capacity(16);

        if let Ok(dir) = fs::read_dir(path) {
            for entry in dir {
                if let Ok(entry) = entry
                    && let Ok(metadata) = entry.metadata()
                {
                    if metadata.is_dir() && entry.file_name() == "wasm" {
                        // load wasm extensions
                        match common::fs::walk_dir_with_ext(entry.path(), ".wasm") {
                            Ok(wasm_paths) => {
                                for wasm_path in wasm_paths {
                                    log::trace!("Loading wasm from {wasm_path:?}");

                                    match self.load_wasm(id, wasm_path) {
                                        Ok(wasm_component) => {
                                            wasm_extensions
                                                .insert(wasm_component.id, wasm_component.clone());
                                            self.register_wasm_component(
                                                wasm_component.id,
                                                wasm_component,
                                            );
                                        }
                                        Err(err) => {
                                            log::error!("{err}")
                                            // TODO error handing
                                        }
                                    }
                                }
                            }
                            Err(err) => log::error!("Failed to walk wasm dir by {err}"),
                        }
                    } else if metadata.is_dir() && entry.file_name() == "wasm-ui" {
                        let _result = common::fs::walk_dir_files(entry.path())
                            .inspect_err(|err| log::error!("Failed to walk wasm dir by {err}"));
                        log::debug!("not yet implemented (wasm-ui)")
                    } else if metadata.is_dir() && entry.file_name() == "defines" {
                        log::debug!("not yet implemented (defines)")
                    } else if metadata.is_dir() && entry.file_name() == "asset" {
                        log::debug!("not yet implemented (asset)")
                    }
                }
            }
        }

        log::info!("Loaded external plugin {name}",);

        let wasm_extensions = match wasm_extensions.len() {
            0 => None,
            _ => Some(wasm_extensions),
        };

        let wasm_ui_extensions = match _wasm_ui_extensions.len() {
            0 => None,
            _ => Some(_wasm_ui_extensions),
        };

        let enable = AtomicBool::new(false);
        let permission = vec![];

        // Free any previously registered pre-load ID for this name
        if let Some(&old_id) = self.plugin_names.get(&name) {
            self.free_plugin_id(old_id);
        }
        self.insert_plugin_name(name.clone(), id);

        Ok(Arc::new(PluginRaw::ExternalPlugin {
            id,
            enable,
            name,
            version,
            description,
            author,
            email,
            permission,
            wasm_extensions,
            wasm_ui_extensions,
            dependencies,
            conflicts,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_load_plugin() {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        let engine = Engine::new(&config).unwrap();
        let mut plugin_loader = PluginLoader::new(engine);
        let lz_plugin = plugin_loader
            .pre_load_plugin_from_path("tests/cosmox_plugin_example")
            .unwrap();

        assert!(matches!(lz_plugin, LazyLoadPlugin::ExternalPlugin { .. }))
    }

    #[test]
    fn test_load_wasm() {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        let engine = Engine::new(&config).unwrap();
        let mut plugin_loader = PluginLoader::new(engine);
        let path = "tests/cosmox_plugin_example/wasm/cosmox_plugin_example.wasm";
        let _wasm_component = plugin_loader.load_wasm(PluginId::new(0), path).unwrap();
    }

    #[test]
    fn test_parse_dependency() {
        let dep = Dependency::parse("my-plugin@>=1.2.3").unwrap();
        assert_eq!(dep.name, PluginName::new("my-plugin"));
        assert_eq!(
            dep.requirement,
            VersionRequirement::GreaterEqual(Version::from("1.2.3"))
        );

        let dep = Dependency::parse("logic-gate@^1").unwrap();
        assert_eq!(
            dep.requirement,
            VersionRequirement::Caret(Version::from("1"))
        );

        let dep = Dependency::parse("ui-kit@~2.5").unwrap();
        assert_eq!(
            dep.requirement,
            VersionRequirement::Tilde(Version::from("2.5"))
        );

        let dep = Dependency::parse("base-lib@3.0.1").unwrap();
        assert_eq!(
            dep.requirement,
            VersionRequirement::Exact(Version::from("3.0.1"))
        );

        let dep = Dependency::parse("legacy-tool@<=0.8.0").unwrap();
        assert_eq!(
            dep.requirement,
            VersionRequirement::LessEqual(Version::from("0.8.0"))
        );
    }

    #[test]
    fn test_parse_dependency_errors() {
        // assert!(parse_dependency("invalid-format").is_err());
        assert!(Dependency::parse("plugin@").is_err());
    }

    /// Helper to convert a "Normal Dep" list (A depends on [B])
    /// into a "Reverse Dep" map (B is a dependency for [A])
    /// and return the total node count.
    fn build_reverse_deps(
        input: Vec<(u64, Vec<u64>)>,
    ) -> (HashMap<PluginId, Vec<PluginId>>, usize) {
        let mut reverse_map: HashMap<PluginId, Vec<PluginId>> = HashMap::new();
        let mut all_ids = std::collections::HashSet::new();

        for (id, deps) in input {
            let id = PluginId::new(id);
            all_ids.insert(id);
            // Ensure the id exists in the map even if nothing depends on it
            reverse_map.entry(id).or_default();

            for dep_id in deps {
                let dep_id = PluginId::new(dep_id);
                all_ids.insert(dep_id);
                // dep_id is the "provider", id is the "consumer"
                reverse_map.entry(dep_id).or_default().push(id);
            }
        }
        (reverse_map, all_ids.len())
    }

    #[test]
    fn test_dependencies_analyzer_simple_linear() {
        // A(1) -> B(2) -> C(3)
        // This means 3 is the base, 2 depends on 3, 1 depends on 2.
        let (deps, total) = build_reverse_deps(vec![(1, vec![2]), (2, vec![3]), (3, vec![])]);

        let result = PluginLoader::dependencies_analyzer(deps, total).unwrap();

        // Expected batches: [[3], [2], [1]]
        assert_eq!(result[0], vec![PluginId::new(3)]);
        assert_eq!(result[1], vec![PluginId::new(2)]);
        assert_eq!(result[2], vec![PluginId::new(1)]);
    }

    #[test]
    fn test_dependencies_analyzer_parallel_batches() {
        // 3 depends on 1 and 2. (1 and 2 are base plugins)
        let (deps, total) = build_reverse_deps(vec![(1, vec![]), (2, vec![]), (3, vec![1, 2])]);

        let result = PluginLoader::dependencies_analyzer(deps, total).unwrap();

        // Batch 0 should have 1 and 2 (order may vary)
        assert_eq!(result.len(), 2);
        assert!(result[0].contains(&PluginId::new(1)));
        assert!(result[0].contains(&PluginId::new(2)));
        assert_eq!(result[1], vec![PluginId::new(3)]);
    }

    #[test]
    fn test_dependencies_analyzer_circular() {
        // 1 -> 2 -> 1
        let (deps, total) = build_reverse_deps(vec![(1, vec![2]), (2, vec![1])]);

        let result = PluginLoader::dependencies_analyzer(deps, total);

        // Should return Err with the IDs in the cycle
        assert!(result.is_err());
        let err_ids = result.unwrap_err();
        assert!(err_ids.iter().any(|x| x.id == PluginId::new(1)));
        assert!(err_ids.iter().any(|x| x.id == PluginId::new(2)));
    }

    #[test]
    fn test_dependencies_analyzer_complex_graph() {
        // 1 -> [2, 3], 2 -> [4], 3 -> [4], 4 -> []
        let (deps, total) = build_reverse_deps(vec![
            (1, vec![2, 3]),
            (2, vec![4]),
            (3, vec![4]),
            (4, vec![]),
        ]);

        let result = PluginLoader::dependencies_analyzer(deps, total).unwrap();

        // Batch 0: [4] (Base)
        // Batch 1: [2, 3]
        // Batch 2: [1]
        assert_eq!(result[0], vec![PluginId::new(4)]);
        assert!(result[1].contains(&PluginId::new(2)));
        assert!(result[1].contains(&PluginId::new(3)));
        assert_eq!(result[2], vec![PluginId::new(1)]);
    }

    #[test]
    fn test_dependencies_analyzer_real_world_complex() {
        let (deps, total) = build_reverse_deps(vec![
            (0, vec![]),
            (8, vec![]),
            (1, vec![0]),
            (2, vec![0]),
            (3, vec![1]),
            (4, vec![1, 2]),
            (5, vec![2]),
            (6, vec![3, 4]),
            (7, vec![4, 5]),
        ]);

        let result = PluginLoader::dependencies_analyzer(deps, total).expect("Analysis failed");

        // Verification
        assert_eq!(result.len(), 4);
        assert!(result[0].contains(&PluginId::new(0)) && result[0].contains(&PluginId::new(8)));
        assert!(result[3].contains(&PluginId::new(6)) && result[3].contains(&PluginId::new(7)));

        let total_processed: usize = result.iter().map(|b| b.len()).sum();
        assert_eq!(total_processed, 9);
    }
}
