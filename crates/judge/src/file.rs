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
  pub fn get_content(&self) -> Vec<u8> {
    let content = match self {
      Self::Memory(m) => m.to_vec(),
      Self::Builtin(b) => b.content.to_vec(),
    };
    return content;
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
