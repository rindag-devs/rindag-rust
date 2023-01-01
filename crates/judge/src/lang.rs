use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{fmt::Display, hash::Hash, str::FromStr};
use thiserror::Error;

use crate::CONFIG;

/// Programming language.
#[derive(Debug, SerializeDisplay, DeserializeFromStr, Clone, PartialEq, Eq, Hash)]
pub struct Lang {
  name: String,
}

impl Lang {
  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn compile_cmd(&self) -> &Vec<String> {
    &CONFIG.lang[&self.name].compile_cmd
  }

  pub fn run_cmd(&self) -> &Vec<String> {
    &CONFIG.lang[&self.name].run_cmd
  }

  pub fn source(&self) -> &str {
    &CONFIG.lang[&self.name].source
  }

  pub fn exec(&self) -> &str {
    &CONFIG.lang[&self.name].exec
  }
}

impl FromStr for Lang {
  type Err = InvalidLangError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match CONFIG.lang.get(s) {
      Some(_x) => Ok(Lang {
        name: s.to_string(),
      }),
      None => Err(Self::Err {
        lang: s.to_string(),
      }),
    }
  }
}

impl Display for Lang {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", &self.name)
  }
}

/// Error when parsing a language name which not in global settings.
#[derive(Error, Debug, Clone)]
#[error("invalid lang: {lang}")]
pub struct InvalidLangError {
  pub lang: String,
}
