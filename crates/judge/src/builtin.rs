use std::{borrow::Cow, fmt::Display, str::FromStr};

use serde_with::{DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

mod pools {
  use rust_embed::RustEmbed;

  /// Testlib source code.
  #[derive(RustEmbed)]
  #[folder = "third_party/testlib/"]
  #[include = "*.cpp"]
  #[include = "*.h"]
  pub struct Testlib;

  /// Builtin checkers.
  #[derive(RustEmbed)]
  #[folder = "third_party/testlib/checkers/"]
  #[include = "*.cpp"]
  #[include = "*.h"]
  pub struct Checker;
}

/// Parsed builtin data.
#[derive(Debug, Clone, SerializeDisplay, DeserializeFromStr)]
pub struct File {
  pool: String,
  path: String,
  content: Cow<'static, [u8]>,
}

impl FromStr for File {
  type Err = FileFromStrError;

  /// Convert a string to builtin file.
  ///
  /// Format: `pool:path/to/file`.
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if let Some((pool, path)) = s.split_once(":") {
      return Ok(Self {
        pool: pool.to_string(),
        path: path.to_string(),
        content: match pool {
          "testlib" => {
            pools::Testlib::get(path).map_or(Err(Self::Err::Path(s.to_string())), |x| Ok(x.data))?
          }
          "checker" => {
            pools::Checker::get(path).map_or(Err(Self::Err::Path(s.to_string())), |x| Ok(x.data))?
          }
          _ => Err(Self::Err::Folder(pool.to_string()))?,
        },
      });
    } else {
      return Err(Self::Err::Format(s.to_string()));
    }
  }
}

impl Display for File {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{}", self.pool, self.path)
  }
}

impl File {
  pub fn as_bytes(&self) -> &[u8] {
    return &self.content;
  }
}

#[derive(Debug, Error, Clone)]
pub enum FileFromStrError {
  #[error("format error: {0}")]
  Format(String),

  #[error("builtin folder not found: {0}")]
  Folder(String),

  #[error("builtin file not found: {0}")]
  Path(String),
}
