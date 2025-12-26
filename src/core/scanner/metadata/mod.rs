use std::{
  collections::HashMap,
  marker::PhantomData,
  sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::core::scanner::metadata;

pub mod getter;
pub mod metadata_service;
