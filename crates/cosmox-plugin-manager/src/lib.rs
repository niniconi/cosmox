pub use wasmtime::component::Resource;

use crate::types::{PluginId, PluginName};

pub mod context;
pub mod plugin_lifecycle;
pub mod plugin_loader;
pub mod plugin_manager;
pub mod types;

pub enum PluginIdentifer {
    Id(PluginId),
    Name(PluginName),
}
