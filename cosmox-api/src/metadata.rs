use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::{
  collections::HashMap,
  marker::PhantomData,
  sync::{Arc, Mutex},
};

#[derive(Debug, Default, Serialize, Deserialize, Decode, Encode)]
pub struct Metadata<T> {
  #[serde(skip)]
  pub _marker: PhantomData<T>,

  pub lid: u64,
  pub rid: u64,
  pub file_size: u64,
  pub file_cnt: u64,
  pub name: String,
  pub alias_name: Vec<String>,
  pub origin_name: String,
  pub description: String,
  pub flags: u32,
  pub metadata_type: MetadataType,
  pub origin: String,

  pub data_file_map_id: u64,
  pub cover_file_map_id: u64,

  pub sub_metadatas: Mutex<Vec<Arc<Metadata<T>>>>,

  pub url: String,

  pub checksum: Vec<u8>,

  pub extend: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Decode, Encode)]
pub enum MetadataType {
  File,
  Directory,
  Network,
  Virtual,
}

impl Default for MetadataType {
  fn default() -> Self {
    Self::Virtual
  }
}
