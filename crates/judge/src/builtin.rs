use std::{borrow::Cow, fmt::Display, str::FromStr};

use regex::Regex;
use rust_embed::RustEmbed;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

#[derive(RustEmbed)]
#[folder = "third_party/testlib/"]
#[include = "*.cpp"]
#[include = "*.h"]
/// Testlib source code.
pub struct Testlib;

/// Builtin checkers.
#[derive(RustEmbed)]
#[folder = "third_party/testlib/checkers/"]
#[include = "*.cpp"]
#[include = "*.h"]
pub struct Checker;

/// A parsed builtin file.
#[derive(Debug, Clone, SerializeDisplay, DeserializeFromStr)]
pub struct File {
  pub folder: String,
  pub path: String,
  pub content: Cow<'static, [u8]>,
}

impl FromStr for File {
  type Err = FileFromStrError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    // Convert a string to builtin file.
    //
    // Format:
    //   folder:path/to/file.txt

    lazy_static! {
      static ref PAT: Regex = Regex::new(r"(?s)^(\w+):(.*)$").unwrap();
    }

    return match PAT.captures(s) {
      Some(cap) => {
        let folder = &cap[1];
        let path = &cap[2];

        Ok(Self {
          folder: folder.to_string(),
          path: path.to_string(),
          content: match folder {
            "testlib" => {
              Testlib::get(path).map_or(Err(Self::Err::Path(s.to_string())), |x| Ok(x.data))?
            }
            "checker" => {
              Checker::get(path).map_or(Err(Self::Err::Path(s.to_string())), |x| Ok(x.data))?
            }
            _ => Err(Self::Err::Folder(folder.to_string()))?,
          },
        })
      }
      None => Err(Self::Err::Format(s.to_string())),
    };
  }
}

impl Display for File {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{}", self.folder, self.path)
  }
}

#[derive(Debug, Error)]
pub enum FileFromStrError {
  #[error("format error: {0}")]
  Format(String),

  #[error("builtin folder not found: {0}")]
  Folder(String),

  #[error("builtin file not found: {0}")]
  Path(String),
}
