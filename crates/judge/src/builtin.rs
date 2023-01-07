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
      return Ok(Self::new(pool, path)?);
    }
    return Err(Self::Err::Format(s.to_string()));
  }
}

impl Display for File {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{}", self.pool, self.path)
  }
}

impl File {
  pub fn new(pool: &str, path: &str) -> Result<Self, FileNotExistError> {
    Ok(Self {
      pool: pool.to_string(),
      path: path.to_string(),
      content: match pool {
        "testlib" => pools::Testlib::get(path),
        "checker" => pools::Checker::get(path),
        _ => return Err(FileNotExistError::Pool(pool.to_string())),
      }
      .map_or(
        Err(FileNotExistError::Path {
          pool: pool.to_string(),
          path: path.to_string(),
        }),
        |x| Ok(x.data),
      )?,
    })
  }

  pub fn as_bytes(&self) -> &[u8] {
    return &self.content;
  }
}

#[derive(Debug, Error, Clone)]
pub enum FileFromStrError {
  #[error("format error: {0}")]
  Format(String),

  #[error("target file can not be found: {0}")]
  NotExist(#[from] FileNotExistError),
}

#[derive(Debug, Error, Clone)]
pub enum FileNotExistError {
  #[error("builtin pool not found: {0}")]
  Pool(String),

  #[error("builtin file not found: `{pool}:{path}`")]
  Path { pool: String, path: String },
}
