use serde::{Deserialize, Serialize};

use crate::builtin;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum File {
  #[serde(with = "serde_bytes")]
  Memory(Vec<u8>),
  Builtin(builtin::File),
}
