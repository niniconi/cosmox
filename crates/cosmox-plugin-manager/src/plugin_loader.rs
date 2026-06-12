use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{fs, path::Path};

use anyhow::Result;
use tracing::{Level, span};
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::*;
// use wasmtime_wasi::p2::bindings::sync::Command;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use bindings::cosmox::plugin::context as bindings_context;
use bindings::cosmox::plugin::cosmox_api as bindings_cosmox_api;
use bindings::cosmox::plugin::cosmox_types as bindings_cosmox_types;
use cosmox_api::{self, Event};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

use crate::plugin_manager::PluginManager;
use crate::types::{
    About, Dependency, Plugin, PluginId, PluginName, PluginRaw, PluginWasmId, PluginWasmName,
    Version, VersionRequirement, WasmComponent, WasmUiExtension,
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

    #[error("Dependency cycle detected involving plugin: {0}")]
    CircularDependency(String),

    #[error("Missing required dependency: {requirement} (required by {name})")]
    MissingDependency {
        name: PluginName,
        requirement: String,
    },

    #[error(
        "Conflict detected: Plugin '{name}' conflicts with '{conflicting_with}'. Reason: {reason}"
    )]
    ConflictDependency {
        name: PluginName,
        conflicting_with: String,
        reason: String,
    },

    #[error("Wasm component '{0}' failed to initialize: {1}")]
    WasmInstantiationError(String, String),

    #[error("IO error while loading plugin: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Plugin metadata is invalid: {0}")]
    InvalidMetadata(String),

    #[error("Missing manifest file (plugin.toml/json).")]
    MissingManifest,

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
    pub bind_events: Mutex<Vec<Arc<cosmox_api::Event>>>,
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

    fn query_host_context(&mut self, _query_key: String, _context_id: String) -> Option<Vec<u8>> {
        unimplemented!("Unimplemented..")
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

/// load wasm plugin from disk
pub fn load_wasm<P>(plugin_id: PluginId, path: P, engine: &Engine) -> Result<Arc<WasmComponent>>
where
    P: AsRef<Path>,
{
    let id = PluginManager::get_wasm_autoincrement();
    let mut linker: Linker<ComponentRunStates> = Linker::new(engine);

    // wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    // wasmtime_wasi::p3::add_to_linker(&mut linker)?;
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
    //   },
    // };

    let component = Component::from_file(engine, &path)?;
    let name = PluginWasmName::new(
        path.as_ref()
            .file_name()
            .map(|x| x.to_str().unwrap().to_string())
            .unwrap_or("".to_string()),
    );

    bindings::cosmox::plugin::cosmox_api::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| {
        &mut s.plugin_data
    })?;

    bindings::cosmox::plugin::context::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;

    let linker = Arc::new(linker);
    let path = PathBuf::from(path.as_ref());

    Ok(Arc::new(WasmComponent {
        id,
        name,
        plugin_id,
        path,
        component,
        linker,
    }))
}

pub fn finalize_dependency() -> Result<(), Vec<PluginLoadError>> {
    let plugin_manager = PluginManager::get_plugin_manager();
    let mut errors = vec![];
    for plugin_id in plugin_manager.plugin_names.values() {
        let Some(plugin) = plugin_manager.plugins[usize::from(plugin_id)].clone() else {
            unreachable!();
        };

        let name = plugin.name();

        let (deps, conflicts) = match plugin.as_ref() {
            PluginRaw::ExternalPlugin {
                dependencies,
                conflicts,
                ..
            } => (dependencies, conflicts),
            PluginRaw::BuiltinPlugin { .. } => (&None, &None),
        };

        if let Some(deps) = deps {
            for dep in deps {
                let Some(plugin_id) = plugin_manager.plugin_names.get(&dep.name) else {
                    log::error!("Plugin {name} is missing dependency {}.", dep.name);
                    errors.push(PluginLoadError::MissingDependency {
                        name: name.clone(),
                        requirement: dep.name.to_string(),
                    });
                    continue;
                };

                let Some(plugin) = plugin_manager.plugins[usize::from(plugin_id)].clone() else {
                    unreachable!();
                };

                let real_version = plugin.version();

                let miss_version = match &dep.requirement {
                    VersionRequirement::Exact(version) => (real_version != version).then(|| {
                        log::error!("Dependency Error: '{name}' requires '{target}' at exactly {version}, but found {real_version}", target = dep.name);
                    }),
                    VersionRequirement::LessEqual(version) => (real_version > version).then(|| {
                        log::error!("Dependency Error: '{name}' requires '{target}' <= {version}, but found {real_version} (too high)", target = dep.name);
                    }),
                    VersionRequirement::GreaterEqual(version) => (real_version < version).then(|| {
                        log::error!("Dependency Error: '{name}' requires '{target}' >= {version}, but found {real_version} (too low)", target = dep.name);
                    }),
                    VersionRequirement::Caret(version) => (!real_version.matches_caret(version)).then(|| {
                        log::error!("Dependency Error: '{name}' requires '{target}' ^{version}, but found {real_version} (incompatible)", target = dep.name);
                    }),
                    VersionRequirement::Tilde(version) => (!real_version.matches_tilde(version)).then(|| {
                        log::error!("Dependency Error: '{name}' requires '{target}' ~{version}, but found {real_version} (patch range mismatch)", target = dep.name);
                    }),
                    VersionRequirement::Any => continue,
                };

                if miss_version.is_some() {
                    errors.push(PluginLoadError::MissingDependency {
                        name: name.clone(),
                        requirement: dep.to_string(),
                    })
                }
            }
        }

        if let Some(conflicts) = conflicts {
            for conflict in conflicts {
                let Some(conflict_plugin_id) = plugin_manager.plugin_names.get(&conflict.name)
                else {
                    continue;
                };

                let Some(conflict_plugin) =
                    plugin_manager.plugins[usize::from(conflict_plugin_id)].clone()
                else {
                    unreachable!();
                };

                let conflict_version = conflict_plugin.version();

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
                    errors.push(PluginLoadError::ConflictDependency {
                        name: name.clone(),
                        conflicting_with: conflict.to_string(),
                        reason: conflict_reason,
                    })
                }
            }
        }
    }
    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok(())
    }
}

/// Analyzes plugin dependencies and returns them in executable batches.
///
/// # Arguments
/// * `reverse_deps` - A map where the key is a Plugin ID and the value is a list of Plugin IDs that depend on it.
/// * `total_nodes` - The total number of unique plugin IDs to be processed.
///
/// # Returns
/// * `Ok(Vec<Vec<u64>>)` - Successive batches of plugin IDs that can be executed in parallel.
/// * `Err(Vec<u64>)` - A list of plugin IDs involved in a circular dependency.
pub fn dependencies_analyzer(
    reverse_deps: HashMap<u64, Vec<u64>>,
    total_nodes: usize,
) -> Result<Vec<Vec<u64>>, Vec<u64>> {
    let mut in_degree: HashMap<u64, usize> = HashMap::with_capacity(total_nodes);

    for &id in reverse_deps.keys() {
        in_degree.entry(id).or_insert(0);
    }

    for neighbors in reverse_deps.values() {
        for &neighbor in neighbors {
            *in_degree.entry(neighbor).or_insert(0) += 1;
        }
    }

    let mut current_batch: Vec<u64> = in_degree
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
        let missing: Vec<u64> = in_degree
            .iter()
            .filter(|&(_, &count)| count > 0)
            .map(|(id, _)| *id)
            .collect();
        return Err(missing);
    }

    Ok(result)
}

pub fn load_builtin_plugins() -> Vec<Plugin> {
    let plugin_id = PluginManager::get_plugin_autoincrement();
    PluginManager::insert_plugin_name(PluginName::new("test"), plugin_id);
    vec![Arc::new(PluginRaw::BuiltinPlugin {
        id: plugin_id,
        version: Version::from(env!("CARGO_PKG_VERSION")),
        name: PluginName::new("test"),
        description: "".to_string(),
    })]
}

pub fn load_external_plugins<P: AsRef<Path> + Debug>(path: P) -> Vec<Plugin> {
    let mut plugins = Vec::new();

    if let Ok(entries) = fs::read_dir(&path) {
        for entry in entries {
            if let Ok(entry) = entry
                && let Ok(metadata) = entry.metadata()
                && metadata.is_dir()
            {
                let plugin = load(entry.path());
                if let Ok(plugin) = plugin {
                    plugins.push(plugin);
                }
            }
        }
    }

    log::trace!("Loaded external plugins from {path:#?}");

    plugins
}

/// load plugin from directory
/// # struct of plugin
/// ```text
/// ```
/// ## about.yaml
/// ```yaml
/// ```
///
/// # Arguments
/// - `path`: the directory of plugin
pub fn load<P: AsRef<Path> + Debug>(path: P) -> Result<Plugin, PluginLoadError> {
    log::trace!("Loading plugin from {path:?}");
    let id = PluginManager::get_plugin_autoincrement();
    let path = path.as_ref();
    let mut about: Option<About> = None;
    let mut wasm_extensions: HashMap<PluginWasmId, Arc<WasmComponent>> = HashMap::with_capacity(16);
    let mut _wasm_ui_extensions: HashMap<PluginWasmId, Arc<WasmUiExtension>> =
        HashMap::with_capacity(16);

    if let Ok(dir) = fs::read_dir(path) {
        for entry in dir {
            if let Ok(entry) = entry
                && let Ok(metadata) = entry.metadata()
            {
                if metadata.is_file() && entry.file_name() == "about.yaml" {
                    // read about.yaml
                    if let Ok(data) = fs::read(path.join("about.yaml")) {
                        about = serde_yaml::from_slice(data.as_slice())
                            .inspect_err(|err| {
                                log::error!("Failed to load about.yaml because {err}")
                            })
                            .unwrap_or(None)
                    }
                } else if metadata.is_dir() && entry.file_name() == "wasm" {
                    // load wasm extensions
                    match common::fs::walk_dir_with_ext(entry.path(), ".wasm") {
                        Ok(wasm_paths) => {
                            for wasm_path in wasm_paths {
                                log::trace!("Loading wasm from {wasm_path:?}");

                                match load_wasm(id, wasm_path, &PluginManager::get_wasm_engine()) {
                                    Ok(wasm_component) => {
                                        wasm_extensions
                                            .insert(wasm_component.id, wasm_component.clone());
                                        PluginManager::register_wasm_component(
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

    if let Some(about) = about {
        log::info!("Loaded external plugin {}", about.name);

        let wasm_extensions = match wasm_extensions.len() {
            0 => None,
            _ => Some(wasm_extensions),
        };

        let wasm_ui_extensions = match _wasm_ui_extensions.len() {
            0 => None,
            _ => Some(_wasm_ui_extensions),
        };

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

        let dependencies = parse_dep(about.dependencies);
        let conflicts = parse_dep(about.conflicts);
        let name = PluginName::new(about.name);
        let version = Version::from(about.version);
        let description = about.description.unwrap_or("".to_string());
        let author = "".to_string();
        let email = about.email.unwrap_or("".to_string());
        let permission = vec![];

        PluginManager::insert_plugin_name(name.clone(), id);
        Ok(Arc::new(PluginRaw::ExternalPlugin {
            id,
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
    } else {
        log::error!("Invalid plugin struct at {path:#?}");
        Err(PluginLoadError::NotFound("".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_load_plugin() {
        let plugin = load("test/plugin/cosmox-plugin-example");
        assert!(plugin.is_ok())
    }

    #[test]
    fn test_load_wasm() {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        let engine = Engine::new(&config).unwrap();
        let path = "test/plugin/cosmox-plugin-example/wasm/cosmox_plugin_example.wasm";
        let _wasm_component = load_wasm(PluginId::new(0), path, &engine).unwrap();
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
    fn build_reverse_deps(input: Vec<(u64, Vec<u64>)>) -> (HashMap<u64, Vec<u64>>, usize) {
        let mut reverse_map: HashMap<u64, Vec<u64>> = HashMap::new();
        let mut all_ids = std::collections::HashSet::new();

        for (id, deps) in input {
            all_ids.insert(id);
            // Ensure the id exists in the map even if nothing depends on it
            reverse_map.entry(id).or_default();

            for dep_id in deps {
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

        let result = dependencies_analyzer(deps, total).unwrap();

        // Expected batches: [[3], [2], [1]]
        assert_eq!(result[0], vec![3]);
        assert_eq!(result[1], vec![2]);
        assert_eq!(result[2], vec![1]);
    }

    #[test]
    fn test_dependencies_analyzer_parallel_batches() {
        // 3 depends on 1 and 2. (1 and 2 are base plugins)
        let (deps, total) = build_reverse_deps(vec![(1, vec![]), (2, vec![]), (3, vec![1, 2])]);

        let result = dependencies_analyzer(deps, total).unwrap();

        // Batch 0 should have 1 and 2 (order may vary)
        assert_eq!(result.len(), 2);
        assert!(result[0].contains(&1));
        assert!(result[0].contains(&2));
        assert_eq!(result[1], vec![3]);
    }

    #[test]
    fn test_dependencies_analyzer_circular() {
        // 1 -> 2 -> 1
        let (deps, total) = build_reverse_deps(vec![(1, vec![2]), (2, vec![1])]);

        let result = dependencies_analyzer(deps, total);

        // Should return Err with the IDs in the cycle
        assert!(result.is_err());
        let err_ids = result.unwrap_err();
        assert!(err_ids.contains(&1));
        assert!(err_ids.contains(&2));
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

        let result = dependencies_analyzer(deps, total).unwrap();

        // Batch 0: [4] (Base)
        // Batch 1: [2, 3]
        // Batch 2: [1]
        assert_eq!(result[0], vec![4]);
        assert!(result[1].contains(&2));
        assert!(result[1].contains(&3));
        assert_eq!(result[2], vec![1]);
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

        let result = dependencies_analyzer(deps, total).expect("Analysis failed");

        // Verification
        assert_eq!(result.len(), 4);
        assert!(result[0].contains(&0) && result[0].contains(&8));
        assert!(result[3].contains(&6) && result[3].contains(&7));

        let total_processed: usize = result.iter().map(|b| b.len()).sum();
        assert_eq!(total_processed, 9);
    }
}
