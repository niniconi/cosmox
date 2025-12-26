use anyhow::{Result, anyhow};
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
  pub name: String,
  pub alias_name: Vec<String>,
  pub origin_name: String,
  pub description: String,
  pub flags: u32,
  pub metadata_type: MetadataType,
  pub origin: String,

  pub data_file_map_id: Option<u64>,
  pub cover_file_map_id: Option<u64>,

  pub sub_metadatas: Vec<Arc<Mutex<Metadata<T>>>>,

  pub url: String,

  pub checksum: Vec<u8>,

  pub extend: HashMap<String, String>,
}

impl<T> Debug for Metadata<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Metadata")
      .field("lid", &self.lid)
      .field("name", &self.name)
      .field("sub", &self.sub_metadatas)
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

    fmt_str.push_str(format!("\'{}\'\n", &metadata.name).as_str());

    let sub_metadatas = &metadata.sub_metadatas;
    if !sub_metadatas.is_empty() {
      for sub in sub_metadatas[..sub_metadatas.len() - 1].iter() {
        let sub = sub.lock().unwrap();
        fmt_str.push_str(Self::_tree_fmt(&*sub, depth + 1, false, over_pos).as_str());
        drop(sub)
      }
      if let Some(last) = sub_metadatas.last() {
        over_pos.insert(depth + 1);
        let last = last.lock().unwrap();
        fmt_str.push_str(Self::_tree_fmt(&*last, depth + 1, true, over_pos).as_str());
        drop(last);
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

#[allow(non_snake_case)]
mod bincode__internal_access {
  use bincode::{enc, error::EncodeError};
  pub struct IoWriter<'a, W: std::io::Write> {
    writer: &'a mut W,
    bytes_written: usize,
  }
  impl<'a, W: std::io::Write> IoWriter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
      Self {
        writer,
        bytes_written: 0,
      }
    }

    pub const fn bytes_written(&self) -> usize {
      self.bytes_written
    }
  }

  impl<W: std::io::Write> bincode::enc::write::Writer for IoWriter<'_, W> {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
      self
        .writer
        .write_all(bytes)
        .map_err(|inner| EncodeError::Io {
          inner,
          index: self.bytes_written,
        })?;
      self.bytes_written += bytes.len();
      Ok(())
    }
  }

  #[derive(Default)]
  pub struct VecWriter {
    pub inner: Vec<u8>,
  }

  impl VecWriter {
    /// Create a new vec writer with the given capacity
    pub fn with_capacity(cap: usize) -> Self {
      Self {
        inner: Vec::with_capacity(cap),
      }
    }
  }

  impl enc::write::Writer for VecWriter {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
      self.inner.extend_from_slice(bytes);
      Ok(())
    }
  }
}

impl<T: Encode> Metadata<T> {
  pub fn binencode(&self) -> Result<Vec<u8>> {
    let config = bincode::config::standard();
    bincode::encode_to_vec(self, config).map_err(|err| anyhow!(err))
  }

  pub fn encode_no_child_into_std_write<W: std::io::Write>(&self, dst: &mut W) -> Result<usize> {
    let config = bincode::config::standard();
    let placeholder = Mutex::<Vec<Arc<Metadata<T>>>>::default();
    let writer = bincode__internal_access::IoWriter::new(dst);
    let mut encoder = bincode::enc::EncoderImpl::<_, _>::new(writer, config);

    bincode::Encode::encode(&self._marker, &mut encoder)?;
    bincode::Encode::encode(&self.lid, &mut encoder)?;
    bincode::Encode::encode(&self.rid, &mut encoder)?;
    bincode::Encode::encode(&self.file_size, &mut encoder)?;
    bincode::Encode::encode(&self.file_cnt, &mut encoder)?;
    bincode::Encode::encode(&self.name, &mut encoder)?;
    bincode::Encode::encode(&self.alias_name, &mut encoder)?;
    bincode::Encode::encode(&self.origin_name, &mut encoder)?;
    bincode::Encode::encode(&self.description, &mut encoder)?;
    bincode::Encode::encode(&self.flags, &mut encoder)?;
    bincode::Encode::encode(&self.metadata_type, &mut encoder)?;
    bincode::Encode::encode(&self.origin, &mut encoder)?;
    bincode::Encode::encode(&self.data_file_map_id, &mut encoder)?;
    bincode::Encode::encode(&self.cover_file_map_id, &mut encoder)?;
    bincode::Encode::encode(&placeholder, &mut encoder)?;
    bincode::Encode::encode(&self.url, &mut encoder)?;
    bincode::Encode::encode(&self.checksum, &mut encoder)?;
    bincode::Encode::encode(&self.extend, &mut encoder)?;

    Ok(encoder.into_writer().bytes_written())
  }

  pub fn encode_no_child_to_vec(&self) -> Result<Vec<u8>> {
    let config = bincode::config::standard();
    let placeholder = Mutex::<Vec<Arc<Metadata<T>>>>::default();

    let size = {
      let mut size_writer =
        bincode::enc::EncoderImpl::<_, _>::new(bincode::enc::write::SizeWriter::default(), config);

      bincode::Encode::encode(&self._marker, &mut size_writer)?;
      bincode::Encode::encode(&self.lid, &mut size_writer)?;
      bincode::Encode::encode(&self.rid, &mut size_writer)?;
      bincode::Encode::encode(&self.file_size, &mut size_writer)?;
      bincode::Encode::encode(&self.file_cnt, &mut size_writer)?;
      bincode::Encode::encode(&self.name, &mut size_writer)?;
      bincode::Encode::encode(&self.alias_name, &mut size_writer)?;
      bincode::Encode::encode(&self.origin_name, &mut size_writer)?;
      bincode::Encode::encode(&self.description, &mut size_writer)?;
      bincode::Encode::encode(&self.flags, &mut size_writer)?;
      bincode::Encode::encode(&self.metadata_type, &mut size_writer)?;
      bincode::Encode::encode(&self.origin, &mut size_writer)?;
      bincode::Encode::encode(&self.data_file_map_id, &mut size_writer)?;
      bincode::Encode::encode(&self.cover_file_map_id, &mut size_writer)?;
      bincode::Encode::encode(&placeholder, &mut size_writer)?;
      bincode::Encode::encode(&self.url, &mut size_writer)?;
      bincode::Encode::encode(&self.checksum, &mut size_writer)?;
      bincode::Encode::encode(&self.extend, &mut size_writer)?;
      size_writer.into_writer().bytes_written
    };
    let writer = bincode__internal_access::VecWriter::with_capacity(size);
    let mut encoder = bincode::enc::EncoderImpl::<_, _>::new(writer, config);
    bincode::Encode::encode(&self._marker, &mut encoder)?;
    bincode::Encode::encode(&self.lid, &mut encoder)?;
    bincode::Encode::encode(&self.rid, &mut encoder)?;
    bincode::Encode::encode(&self.file_size, &mut encoder)?;
    bincode::Encode::encode(&self.file_cnt, &mut encoder)?;
    bincode::Encode::encode(&self.name, &mut encoder)?;
    bincode::Encode::encode(&self.alias_name, &mut encoder)?;
    bincode::Encode::encode(&self.origin_name, &mut encoder)?;
    bincode::Encode::encode(&self.description, &mut encoder)?;
    bincode::Encode::encode(&self.flags, &mut encoder)?;
    bincode::Encode::encode(&self.metadata_type, &mut encoder)?;
    bincode::Encode::encode(&self.origin, &mut encoder)?;
    bincode::Encode::encode(&self.data_file_map_id, &mut encoder)?;
    bincode::Encode::encode(&self.cover_file_map_id, &mut encoder)?;
    bincode::Encode::encode(&placeholder, &mut encoder)?;
    bincode::Encode::encode(&self.url, &mut encoder)?;
    bincode::Encode::encode(&self.checksum, &mut encoder)?;
    bincode::Encode::encode(&self.extend, &mut encoder)?;
    Ok(encoder.into_writer().inner)
  }
}

impl<T: Decode<()>> Metadata<T> {
  pub fn bindecode(data: Vec<u8>) -> Result<Arc<Metadata<T>>> {
    let config = bincode::config::standard();
    Ok(
      bincode::decode_from_slice::<Arc<Metadata<T>>, _>(&data, config)
        .map(|(metadata, _)| metadata)?,
    )
  }

  pub fn bindecode_from<R: std::io::Read>(src: &mut R) -> Result<Arc<Mutex<Metadata<T>>>> {
    let config = bincode::config::standard();
    Ok(bincode::decode_from_std_read::<Arc<Mutex<Metadata<T>>>, _, _>(
      src, config,
    )?)
  }

  pub fn bindecode_set_id(data: Vec<u8>, id: u64) -> Result<Arc<Mutex<Metadata<T>>>> {
    let config = bincode::config::standard();
    bincode::decode_from_slice::<Arc<Mutex<Metadata<T>>>, _>(&data, config)
      .map(|(metadata, _)| {
        let mut gurad = metadata.lock().unwrap();
        gurad.rid = id;
        drop(gurad);
        metadata
      })
      .map_err(|err| anyhow!(err))
  }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Decode, Encode)]
pub enum MetadataType {
  File,
  Directory,
  Network,
  #[default]
  Virtual,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn test_encode_no_child_ne() {
    let config = bincode::config::standard();

    let mut metadata = Metadata::<()> {
      name: "Hi here is metadata's name".to_string(),
      description: "Hi here is metadata's description".to_string(),
      ..Default::default()
    };
    for _ in 0..32 {
      let child = Metadata::<()> {
        name: "child".to_string(),
        ..Default::default()
      };
      metadata.sub_metadatas.push(Arc::new(Mutex::new(child)));
    }
    let data_offical = bincode::encode_to_vec(&metadata, config).unwrap();
    let data_custom = metadata.encode_no_child_to_vec().unwrap();
    assert_ne!(data_offical, data_custom);
    assert_ne!(data_offical.len(), data_custom.len());
  }

  #[test]
  pub fn test_encode_no_child_eq() {
    let config = bincode::config::standard();
    let metadata = Metadata::<()> {
      name: "Hi here is metadata's name".to_string(),
      description: "Hi here is metadata's description".to_string(),
      ..Default::default()
    };
    let data_offical = bincode::encode_to_vec(&metadata, config).unwrap();
    let data_custom = metadata.encode_no_child_to_vec().unwrap();
    assert_eq!(data_offical, data_custom);
    assert_eq!(data_offical.len(), data_custom.len());
  }

  #[test]
  pub fn test_encode_no_child_io_eq() {
    let config = bincode::config::standard();
    let metadata = Metadata::<()> {
      name: "Hi here is metadata's name".to_string(),
      description: "Hi here is metadata's description".to_string(),
      ..Default::default()
    };
    let mut data_offical = Vec::<u8>::new();
    let mut data_custom = Vec::<u8>::new();
    let data_offical_cnt =
      bincode::encode_into_std_write(&metadata, &mut data_offical, config).unwrap();
    let data_custom_cnt = metadata
      .encode_no_child_into_std_write(&mut data_custom)
      .unwrap();
    assert_eq!(data_offical, data_custom);
    assert_eq!(data_offical.len(), data_custom.len());
    assert_eq!(data_offical_cnt, data_custom_cnt);
  }
}
