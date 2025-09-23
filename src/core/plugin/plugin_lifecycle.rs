use wasmtime::Store;

use super::plugin_loader::bindings::cosmox::plugin::cosmox_api as bindings_cosmox_api;
use super::plugin_loader::bindings::cosmox::plugin::cosmox_types as bindings_cosmox_types;
use crate::core::plugin::plugin_manager::PLUGIN_MANAGER;
use crate::core::plugin::{
  plugin_loader::{ComponentRunStates, bindings::PluginHostWorld},
  plugin_manager::PluginManager,
};

pub fn plugin_wasm_lifecycle(store: &mut Store<ComponentRunStates>, instance: PluginHostWorld) {
  log::debug!("lifecycle............................");
  let supported_media_types = instance
    .cosmox_plugin_configuration_manager()
    .call_supported_media_types(&mut *store)
    .unwrap_or_default();

  log::debug!("....................................");
  // log::debug!("try_lock {:?}", PLUGIN_MANAGER.try_lock());
  let _ = instance
    .cosmox_plugin_core_lifecycle()
    .call_on_load(
      &mut *store,
      &bindings_cosmox_types::ConfigData {
        id: String::default(),
        name: String::default(),
        settings: String::default(),
      },
    )
    .unwrap();

  log::debug!("end..................................")
}
