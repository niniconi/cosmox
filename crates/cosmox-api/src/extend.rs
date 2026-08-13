use std::collections::HashMap;

use crate::metadata::Metadata;

/// `#[derive(MetadataExtend)]` for extension metadata structs.
pub use cosmox_macros::MetadataExtend;

/// Constraints for plugin-defined extension metadata.
///
/// A struct implementing this trait is the `T` of `Metadata<T>`; each of
/// its top-level fields expands to one flat `extend` key:
/// `EXTEND_KEY:field` -> the field value as a plain string.
/// Prefer `#[derive(cosmox_macros::MetadataExtend)]` to generate the
/// per-field conversion code.
pub trait MetadataExtend: Default + 'static {
    /// Extend key prefix (e.g. "anime"); the field `season_number`
    /// expands to the key `anime:season_number`.
    const EXTEND_KEY: &'static str;

    /// Consume `self`, expanding it into flat key/value pairs
    /// (move semantics: `String` fields are moved, never cloned).
    fn to_extend_pairs(self) -> Vec<(String, String)>;

    /// Rebuild the struct from flat key/value pairs (only `EXTEND_KEY:`
    /// prefixed keys are collected; missing fields fall back to
    /// `Default` / `Option::None`).
    fn from_extend_pairs(pairs: &HashMap<String, String>) -> Result<Self, ExtendError>;
}

/// Field expansion/aggregation errors.
#[derive(Debug, thiserror::Error)]
pub enum ExtendError {
    /// A string failed to parse into a field value (e.g. "abc" → u32).
    #[error("parse extend field failed: {0}")]
    Parse(String),
    /// The key table does not correspond to an `extend` key on read.
    #[error("extend key missing: {0}")]
    KeyMissing(String),
    /// bincode decode failed (`read_extend` whole-tree decode).
    #[error("decode extend data failed: {0}")]
    Decode(String),
    /// bincode encode failed (`write_extend` field encode).
    #[error("encode extend data failed: {0}")]
    Encode(String),
}

impl<T: MetadataExtend> Metadata<T> {
    /// Aggregate the plugin-defined extension metadata out of `extend`.
    pub fn extend_data(&self) -> Result<Option<T>, ExtendError> {
        Ok(Some(T::from_extend_pairs(&self.extend)?))
    }

    /// Expand the plugin-defined metadata into `extend` (per-field;
    /// consumes `data`, move semantics).
    pub fn set_extend_data(&mut self, data: T) -> Result<(), ExtendError> {
        for (k, v) in data.to_extend_pairs() {
            self.extend.insert(k, v);
        }
        Ok(())
    }
}
