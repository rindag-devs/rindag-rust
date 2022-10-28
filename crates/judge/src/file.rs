use serde::{Deserialize, Serialize};

use crate::builtin;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum File {
  #[serde(with = "serde_bytes")]
  Memory(Vec<u8>),
  Builtin(builtin::File),
}

impl File {
  pub fn as_bytes(&self) -> &[u8] {
    match self {
      Self::Memory(m) => &m,
      Self::Builtin(b) => &b.as_bytes(),
    }
  }
}

impl From<builtin::File> for File {
  fn from(f: builtin::File) -> Self {
    Self::Builtin(f)
  }
}

impl From<Vec<u8>> for File {
  fn from(f: Vec<u8>) -> Self {
    Self::Memory(f)
  }
}
