use std::{collections::HashMap, fmt::Debug, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use wasmtime::component::{Component, Linker};

use crate::core::plugin::plugin_loader::ComponentRunStates;

pub mod plugin_controller;
pub mod plugin_lifecycle;
pub mod plugin_loader;
pub mod plugin_manager;

#[derive(Debug, Clone)]
pub enum Plugin {
  ExternalPlugin {
    id: u64,
    name: String,
    description: String,
    author: String,
    email: String,
    permission: Vec<String>,
    wasm_extensions: Option<HashMap<u64, Arc<WasmComponent>>>,
    wasm_ui_extensions: Option<HashMap<u64, Arc<WasmUiExtension>>>,
  },

  BuiltinPlugin {
    id: u64,
    name: String,
    description: String,
  },
}

impl Plugin {
  pub fn id(&self) -> u64 {
    match self {
      Self::BuiltinPlugin { id, .. } => *id,
      Self::ExternalPlugin { id, .. } => *id,
    }
  }
}

pub struct WasmComponent {
  pub id: u64,
  pub plugin_id: u64,
  pub path: PathBuf,
  pub compoent: Component,
  pub linker: Arc<Linker<ComponentRunStates>>,
}

impl Debug for WasmComponent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "WasmComponent {{ id: {}, plugin_id: {} }}",
      self.id, self.plugin_id
    )
  }
}

#[derive(Debug)]
pub struct WasmUiExtension {
  pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct About {
  pub name: String,
  pub version: String,
  pub license: String,
  pub authors: Vec<String>,
  pub description: Option<String>,
  pub email: Option<String>,
  pub permission: Option<Vec<String>>,
  pub url: Option<String>,
  pub dependencies: Option<Vec<String>>,
}
