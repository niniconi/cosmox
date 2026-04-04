use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Metadata {
  pub dependencies: Option<Vec<String>>,
  pub conflicts: Option<Vec<String>>,
}
