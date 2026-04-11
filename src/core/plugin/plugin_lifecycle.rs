use wasmtime::Store;

use super::plugin_loader::bindings::cosmox::plugin::cosmox_types as bindings_cosmox_types;
use crate::core::plugin::{
  plugin_loader::{ComponentRunStates, bindings::PluginHostWorld},
  plugin_manager::PluginManager,
};

pub async fn plugin_wasm_lifecycle(
  store: &mut Store<ComponentRunStates>,
  instance: PluginHostWorld,
) {
  log::debug!("lifecycle............................");

  let supported_media_types = instance
    .cosmox_plugin_configuration_manager()
    .call_supported_media_types(&mut *store)
    .await
    .unwrap_or_default();
  if let Err(err) = PluginManager::push_media_types(supported_media_types).await {
    log::error!("{err:?}");
  }

  log::debug!("....................................");
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
    .await;

  log::debug!("end..................................")
}
