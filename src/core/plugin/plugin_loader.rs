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

use crate::core::ai::call_llm;
use crate::core::plugin::plugin_manager::PluginManager;
use crate::core::plugin::{About, Plugin};
use crate::core::plugin::{WasmComponent, WasmUiExtension};
use crate::utils;

pub mod bindings {
  pub use super::super::context::event::{MetadataContext, PathMappingContext, TagContext};
  use wasmtime::component::bindgen;

  bindgen!({
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
  #[error("Not found plugin at ")]
  NotFound(String),
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
  pub plugin_id: u64,
  pub wasm_id: u64,
  pub name: String,
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

        if let Err(err) = PluginManager::bind_event_for_wasm(event.into_key(), self.wasm_id) {
          return Err(bindings_cosmox_api::ListenerRegistrationError::Unknown);
        }

        log::info!("registerd {event:?} for wasm {}", self.wasm_id);
        self.bind_events.lock().unwrap().push(event.clone()); // TODO Optimize this
        Ok(())
      }
      Err(err) => {
        log::error!("{err}");
        Err(
          bindings_cosmox_api::ListenerRegistrationError::EventPayloadDecodeError(err.to_string()),
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

        if let Err(err) = PluginManager::unbind_event_from_wasm(event.into_key(), self.wasm_id) {
          return Err(bindings_cosmox_api::ListenerRegistrationError::Unknown);
        }
        Ok(())
      }
      Err(err) => {
        log::error!("{err}");
        Err(
          bindings_cosmox_api::ListenerRegistrationError::EventPayloadDecodeError(err.to_string()),
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
pub fn load_wasm<P>(plugin_id: u64, path: P, engine: &Engine) -> Result<Arc<WasmComponent>>
where
  P: AsRef<Path>,
{
  let wasm_id = PluginManager::gen_wasm_autoincrement();
  let mut linker: Linker<ComponentRunStates> = Linker::new(engine);

  // wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
  wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
  // wasmtime_wasi::p3::add_to_linker(&mut linker)?;
  wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)?;

  // Create a WASI context and put it in a Store; all instances in the store
  // share this context. `WasiCtxBuilder` provides a number of ways to
  // configure what the target program will have access to.
  // let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();
  // let state = ComponentRunStates {
  //   wasi_ctx: wasi,
  //   resource_table: ResourceTable::new(),
  //   plugin_data: CosmoxPluginData {
  //     bind_events: Mutex::new(Vec::with_capacity(32)),
  //     plugin_id: plugin_id,
  //     wasm_id: wasm_id,
  //   },
  // };

  let component = Component::from_file(engine, &path)?;
  let name = path
    .as_ref()
    .file_name()
    .map(|x| x.to_str().unwrap().to_string())
    .unwrap_or("".to_string());

  bindings::cosmox::plugin::cosmox_api::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| {
    &mut s.plugin_data
  })?;

  bindings::cosmox::plugin::context::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;

  Ok(Arc::new(WasmComponent {
    id: wasm_id,
    name: name,
    plugin_id: plugin_id,
    path: PathBuf::from(path.as_ref()),
    compoent: component,
    linker: Arc::new(linker),
  }))
}

pub fn load_builtin_plugins() -> Vec<Plugin> {
  let plugin_id = PluginManager::get_plugin_autoincrement();
  vec![Plugin::BuiltinPlugin {
    id: plugin_id,
    name: "test".to_string(),
    description: "".to_string(),
  }]
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
  let plugin_id = PluginManager::get_plugin_autoincrement();
  let path = path.as_ref();
  let mut about: Option<About> = None;
  let mut wasm_extensions: HashMap<u64, Arc<WasmComponent>> = HashMap::with_capacity(16);
  let mut _wasm_ui_extensions: HashMap<u64, Arc<WasmUiExtension>> = HashMap::with_capacity(16);

  if let Ok(dir) = fs::read_dir(path) {
    for entry in dir {
      if let Ok(entry) = entry
        && let Ok(metadata) = entry.metadata()
      {
        if metadata.is_file() && entry.file_name() == "about.yaml" {
          // read about.yaml
          if let Ok(data) = fs::read(path.join("about.yaml")) {
            about = serde_yaml::from_slice(data.as_slice())
              .inspect_err(|err| log::error!("Failed to load about.yaml because {err}"))
              .unwrap_or(None)
          }
        } else if metadata.is_dir() && entry.file_name() == "wasm" {
          // load wasm extensions
          match utils::fs::walk_dir_with_ext(entry.path(), ".wasm") {
            Ok(wasm_paths) => {
              for wasm_path in wasm_paths {
                log::trace!("Loading wasm from {wasm_path:?}");

                match load_wasm(plugin_id, wasm_path, &PluginManager::get_wasm_engine()) {
                  Ok(wasm_component) => {
                    wasm_extensions.insert(wasm_component.id, wasm_component.clone());
                    PluginManager::register_wasm_component(wasm_component.id, wasm_component);
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
          let _result = utils::fs::walk_dir_files(entry.path())
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

    Ok(Plugin::ExternalPlugin {
      id: plugin_id,
      name: about.name,
      description: about.description.unwrap_or("".to_string()),
      author: "".to_string(),
      email: about.email.unwrap_or("".to_string()),
      permission: vec!["".to_string()],
      wasm_extensions: wasm_extensions,
      wasm_ui_extensions: wasm_ui_extensions,
    })
  } else {
    log::error!("Invalid plugin struct at {path:#?}");
    Err(PluginLoadError::NotFound("".to_string()))
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_load_plugin() {
    let plugin = load("test/plugin/build");
    assert!(plugin.is_ok())
  }

  #[test]
  fn test_load_wasm() {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config).unwrap();
    let path = "test/plugin/build/wasm/cosmox_plugin_example.wasm";
    let _wasm_component = load_wasm(0, path, &engine).unwrap();
  }
}
