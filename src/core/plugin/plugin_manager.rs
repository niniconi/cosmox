use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  sync::{Arc, LazyLock, Mutex},
};

use ffmpeg_next::ffi::printf;
use futures::stream::Collect;
use lru::LruCache;
use regex::Regex;
use sea_orm::sea_query::IdenList;
use wasmtime::component::ResourceTable;
use wasmtime::{Engine, Store};

use super::plugin_loader::bindings::cosmox::plugin::cosmox_types as bindings_cosmox_types;

use crate::{
  configuration::Configuration,
  core::plugin::{
    Plugin, WasmComponent,
    plugin_lifecycle::plugin_wasm_lifecycle,
    plugin_loader::{
      ComponentRunStates, CosmoxPluginData, bindings, load_builtin_plugins, load_external_plugins,
    },
  },
  utils::default_constants::ascii_letters_number_separators,
};

// use tokio::sync::{Mutex, mpsc};
use wasmtime_wasi::WasiCtxBuilder;

#[derive(Default, Debug, Clone)]
pub struct PluginManager {
  /// wasm runtime engine
  pub engine: Arc<Engine>,

  pub plugin_enable_count: usize,
  pub plugin_count: usize,

  pub plugins: HashMap<u64, Plugin>,

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

// --- `thread_local!` Store management ---
// Each Tokio blocking thread will have its own copy of the Store.
thread_local! {
  static WASM_STORE_AND_STATE: LazyLock<RefCell<LruCache<u64, Store<ComponentRunStates>>>> = LazyLock::new(|| {
    log::info!("[Thread {:?}] Initializing LRU Cache for Stores. Thread ID: {:?}",
      std::thread::current().id(), std::thread::current().id());
    // Sets the maximum capacity of the LRU cache. For example, each thread retains a maximum of 10 Store instances.
    // If you need more Store instances to be active simultaneously on the same thread, you can adjust this number.
    RefCell::new(LruCache::new(std::num::NonZeroUsize::new(10).unwrap()))
  });
}

static PLUGIN_MANAGER: LazyLock<Mutex<PluginManager>> =
  LazyLock::new(|| Mutex::new(PluginManager::default()));

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

  /// Generate a wasm_id from plugin manager
  /// ```rust
  /// let wasm_id0 = PluginManager::get_wasm_autoincrement();
  /// let wasm_id1 = PluginManager::get_wasm_autoincrement();
  /// assert_ne!(wasm_id0, wasm_id1);
  /// ```
  #[inline]
  pub fn gen_wasm_autoincrement() -> u64 {
    PLUGIN_MANAGER.lock().unwrap()._gen_wasm_autoincrement()
  }

  fn _gen_wasm_autoincrement(&mut self) -> u64 {
    self.wasm_autoincrement += 1;
    self.wasm_autoincrement
  }

  pub fn start() {
    {
      let plugin_path = &Configuration::get_global_configuration().cosmox.plugin.path;

      let builtin_plugins = load_builtin_plugins();
      let external_plugins = load_external_plugins(plugin_path);

      let mut plugin_manager = PLUGIN_MANAGER.lock().unwrap();

      plugin_manager._start(builtin_plugins, external_plugins);

      log::info!("initialized plugin manager");
    } // Leaving the life cycle of `plugin_manager`

    // The lock of `plugin_manager` has been released
    PluginManager::lifecycle_manager();
  }

  fn _start(&mut self, builtin_plugins: Vec<Plugin>, external_plugins: Vec<Plugin>) {
    self.plugin_count += builtin_plugins.len() + external_plugins.len();
    for plugin in builtin_plugins {
      self.plugins.insert(plugin.id(), plugin);
    }

    for plugin in external_plugins {
      self.plugins.insert(plugin.id(), plugin);
    }
  }

  fn lifecycle_manager() {
    let (engine, wasm_list) = {
      let plugin_manager = PLUGIN_MANAGER.lock().unwrap();
      let wasm_list = plugin_manager.wasm_list.clone();
      let engine = plugin_manager.engine.clone();
      (engine, wasm_list)
    };

    for (wasm_id, wasm_component) in wasm_list.iter() {
      log::debug!(
        "start lifetime manager for wasm ID:{wasm_id}, wasm_component:{wasm_component:#?}"
      );
      WASM_STORE_AND_STATE.with(|cache_ref_cell| {
        let mut cache = cache_ref_cell.borrow_mut();
        let cache_size = cache.len();
        let store_mut_ref = cache.get_or_insert_mut(*wasm_id, || {
          // log::trace!("[Thread '{}' (ID: {:?})] Lazily initializing Store for ServiceKey {}. Cache size: {}",
          // current_thread_name, current_thread_id, task_service_key, cache_size);

          let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();
          let state = ComponentRunStates {
            wasi_ctx: wasi,
            resource_table: ResourceTable::new(),
            plugin_data: CosmoxPluginData {
              bind_events: Mutex::new(Vec::with_capacity(32)),
              plugin_id: wasm_component.plugin_id,
              wasm_id: wasm_component.id,
            },
          };

          Store::new(&engine, state)
        });

        let instance = bindings::PluginHostWorld::instantiate(
          &mut *store_mut_ref,
          &wasm_component.compoent,
          &wasm_component.linker,
        )
        .unwrap();

        plugin_wasm_lifecycle(store_mut_ref, instance);
      })
    }
  }

  pub fn install() {}

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
  pub async fn notify_all(event: Arc<cosmox_api::Event>) {
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
        None => return,
      };
      (components_for_current_event, engine)
    };

    let payload = match event.encode() {
      Ok(payload) => payload,
      Err(_) => return,
    };

    let mut join_handles = Vec::with_capacity(components_for_current_event.len());
    for current_wasm_compoent in components_for_current_event {
      let current_wasm_compoent = current_wasm_compoent.clone();
      let task_service_key = current_wasm_compoent.id;
      let payload = payload.clone();
      let engine = engine.clone();

      let handle = tokio::task::spawn_blocking(move || {
        let payload = payload;
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

        WASM_STORE_AND_STATE.with(|cache_ref_cell| {
            let mut cache = cache_ref_cell.borrow_mut();
            let cache_size = cache.len();

            let store_mut_ref = cache.get_or_insert_mut(task_service_key, || {
                log::trace!("[Thread '{}' (ID: {:?})] Lazily initializing Store for ServiceKey {}. Cache size: {}",
                current_thread_name, current_thread_id, task_service_key, cache_size);

                let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();
                let state = ComponentRunStates {
                    wasi_ctx: wasi,
                    resource_table: ResourceTable::new(),
                    plugin_data:  CosmoxPluginData{
                        bind_events: Mutex::new(Vec::with_capacity(32)),
                        plugin_id: current_wasm_compoent.plugin_id,
                        wasm_id: current_wasm_compoent.id,
                    },
                };

                Store::new(&engine, state)
            });

            let instance = bindings::PluginHostWorld::instantiate(&mut *store_mut_ref,&current_wasm_compoent.compoent, &current_wasm_compoent.linker).unwrap();


            log::debug!("call on_event wasm = {current_wasm_compoent:?}");
            let result = instance.cosmox_plugin_host_notifier().call_on_event(store_mut_ref, payload.clone().as_slice());

            match result {
                Ok(plugin_result) =>  {
                    log::info!("[Host] Wasm Task '{}' (Service {}) completed on thread '{}' (ID: {:?}). Result: {:?}. Current cache size: {}",
                    current_task_name, task_service_key, current_thread_name, current_thread_id, plugin_result, cache.len());

                }
                Err(err) => {log::error!("{err}")}
            }
        });
      });
      join_handles.push(handle);
    }

    for handle in join_handles {
      let _ = handle.await;
    }
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
  pub fn push_media_types(
    media_types: Vec<String>,
  ) -> Result<(), bindings_cosmox_types::MediaTypeError> {
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

    for media_type in &media_types {
      plugin_manager
        .supported_media_types
        .insert(media_type.clone());
    }

    log::info!("Add media types {media_types:?} successful.");

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
mod test {
  use super::*;

  #[test]
  fn plugin_manager_entity() {
    println!("{:#?}", PLUGIN_MANAGER.lock().unwrap());
  }
}
