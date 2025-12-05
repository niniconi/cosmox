use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::{
  collections::{HashMap, HashSet},
  fmt::Debug,
  marker::PhantomData,
  sync::{Arc, Mutex},
};

#[derive(Default, Serialize, Deserialize, Decode, Encode)]
pub struct Metadata<T> {
  #[serde(skip)]
  pub _marker: PhantomData<T>,

  pub lid: u64,
  pub rid: u64,
  pub file_size: u64,
  pub file_cnt: u64,
  pub name: Mutex<String>,
  pub alias_name: Mutex<Vec<String>>,
  pub origin_name: Mutex<String>,
  pub description: Mutex<String>,
  pub flags: u32,
  pub metadata_type: MetadataType,
  pub origin: Mutex<String>,

  pub data_file_map_id: u64,
  pub cover_file_map_id: u64,

  pub sub_metadatas: Mutex<Vec<Arc<Metadata<T>>>>,

  pub url: Mutex<String>,

  pub checksum: Vec<u8>,

  pub extend: HashMap<String, String>,
}

impl<T> Debug for Metadata<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Metadata")
      .field("lid", &self.lid)
      .field("name", &self.name.lock().unwrap())
      .field("sub", &self.sub_metadatas.lock().unwrap())
      .field("type", &self.metadata_type)
      .finish()
  }
}

impl<T> Metadata<T> {
  fn _tree_fmt(
    metadata: &Self,
    depth: usize,
    is_last: bool,
    over_pos: &mut HashSet<usize>,
  ) -> String {
    let mut fmt_str = String::default();
    for idx in 1..depth {
      if over_pos.get(&idx).is_none() {
        fmt_str.push_str("│  ");
      } else {
        fmt_str.push_str("   ");
      }
    }

    if depth != 0 {
      if is_last {
        fmt_str.push_str("└─");
      } else {
        fmt_str.push_str("├─");
      }
    }

    fmt_str.push_str(format!("\'{}\'\n", &metadata.name.lock().unwrap()).as_str());

    let sub_metadatas = metadata.sub_metadatas.lock().unwrap();
    if !sub_metadatas.is_empty() {
      for sub in sub_metadatas[..sub_metadatas.len() - 1].iter() {
        fmt_str.push_str(Self::_tree_fmt(sub, depth + 1, false, over_pos).as_str());
      }
      if let Some(last) = sub_metadatas.last() {
        over_pos.insert(depth + 1);
        fmt_str.push_str(Self::_tree_fmt(last, depth + 1, true, over_pos).as_str());
        over_pos.remove(&(depth + 1));
      }
    }

    fmt_str
  }

  /// Format the metadata tree into a tree structure,
  /// like the example below:
  /// ```text
  ///'Root Node'
  ///└─'Node 1'
  ///   ├─'Node 11'
  ///   │  ├─'Node 111'
  ///   │  ├─'Node 112'
  ///   │  ├─'Node 113'
  ///   │  ├─'Node 114'
  ///   │  ├─'Node 115'
  ///   │  ├─'Node 116'
  ///   │  └─'Node 117'
  ///   ├─'Node 12'
  ///   │  ├─'Node 121'
  ///   │  ├─'Node 122'
  ///   │  ├─'Node 123'
  ///   │  └─'Node 124'
  ///   ├─'Node 13'
  ///   ├─'Node 14'
  ///   └─'Node 15'
  ///```
  pub fn tree_fmt(&self) -> String {
    let mut over_pos = HashSet::new();
    Self::_tree_fmt(self, 0, true, &mut over_pos)
  }
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
