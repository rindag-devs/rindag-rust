use serde::{Deserialize, Serialize};
use std::{
  borrow::Borrow,
  collections::HashSet,
  fmt::Display,
  hash::{Hash, Hasher},
  str::FromStr,
  time,
};
use thiserror::Error;

use crate::ARGS;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
/// Rindag server config.
pub struct Cfg {
  /// The address for the Rindag http server to listen on.
  pub host: String,

  /// Judge token secret.
  ///
  /// Set to `None` to disable auth.
  ///
  /// WARNING: Be sure to set a token secret in a production environment.
  pub secret: Option<String>,

  pub lang: HashSet<LangCfg>,

  pub sandbox: SandboxCfg,
}

impl Default for Cfg {
  // Set default values for config
  fn default() -> Self {
    return Self {
      host: ":8080".to_string(),
      secret: None,
      lang: HashSet::from([
        LangCfg {
          name: "c".to_string(),
          compile_cmd: [
            "/usr/bin/gcc",
            "foo.c",
            "-o",
            "foo",
            "-O2",
            "-w",
            "-fmax-errors=3",
            "-DONLINE_JUDGE",
          ]
          .iter()
          .map(|&s| s.into())
          .collect(),
          run_cmd: vec!["foo".to_string()],
          source: "foo.c".to_string(),
          exec: "foo".to_string(),
        },
        LangCfg {
          name: "cpp".to_string(),
          compile_cmd: [
            "/usr/bin/g++",
            "foo.cpp",
            "-o",
            "foo",
            "-O2",
            "-w",
            "-fmax-errors=3",
            "-DONLINE_JUDGE",
          ]
          .iter()
          .map(|&s| s.into())
          .collect(),
          run_cmd: vec!["foo".to_string()],
          source: "foo.cpp".to_string(),
          exec: "foo".to_string(),
        },
      ]),
      sandbox: SandboxCfg {
        env: vec![
          "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
          "HOME=/w".to_string(),
          "ONLINE_JUDGE=rindag".to_string(),
        ],
        time_limit: time::Duration::from_secs(10),
        memory_limit: 1024 * 1024 * 1024, // 1 GB
        process_limit: 16,                // 16 processes
        stdout_limit: 512 * 1024 * 1024,  // 512 MB
        stderr_limit: 16 * 1024,          // 16 kB
        host: "http://localhost:5051".to_string(),
        max_job: 2,
      },
    };
  }
}

/// Programming language config.
#[derive(Debug, Serialize, Deserialize, Clone, Eq)]
pub struct LangCfg {
  name: String,

  pub compile_cmd: Vec<String>,

  pub run_cmd: Vec<String>,

  /// Name of source file
  pub source: String,

  /// Name of executable file
  pub exec: String,
}

impl LangCfg {
  pub fn name(&self) -> &str {
    return &self.name;
  }
}

impl PartialEq for LangCfg {
  fn eq(&self, other: &LangCfg) -> bool {
    self.name == other.name
  }
}

impl Hash for LangCfg {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.name.hash(state);
  }
}

impl Borrow<str> for LangCfg {
  fn borrow(&self) -> &str {
    &self.name
  }
}

#[derive(Error, Debug, Clone)]
#[error("invalid lang: {lang}")]
/// Error when parsing a language name which not in global settings.
pub struct InvalidLangError {
  pub lang: String,
}

impl FromStr for LangCfg {
  type Err = InvalidLangError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match CONFIG.lang.get(s) {
      Some(x) => Ok(x.clone()),
      None => Err(Self::Err {
        lang: s.to_string(),
      }),
    }
  }
}

impl Display for LangCfg {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", &self.name)
  }
}

/// Sandbox config.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SandboxCfg {
  /// Environment variables.
  pub env: Vec<String>,

  /// CPU time limits for compilation non-solution programs running
  /// such as checkers, validators, generators, etc.
  pub time_limit: time::Duration,

  /// Memory limit for compilation and running non-solution programs in bytes.
  pub memory_limit: u64,

  /// Default process count limit.
  pub process_limit: u64,

  /// Default stdout limit, in bytes.
  pub stdout_limit: i64,

  /// Default stderr limit, in bytes.
  pub stderr_limit: i64,

  /// Sandbox gRpc server host address.
  pub host: String,

  /// Max job count running in the same time.
  pub max_job: usize,
}

impl Cfg {
  /// Create and load the config.
  pub fn load(search_paths: &Vec<String>) -> Self {
    let mut builder = config::Config::builder()
      .add_source(config::File::with_name("/etc/rindag/judge").required(false));

    for p in search_paths {
      builder = builder.add_source(config::File::with_name(p.as_str()).required(false));
    }

    builder = builder.add_source(config::Environment::with_prefix("RINDAG_JUDGE"));

    return builder.build().unwrap().try_deserialize::<Self>().unwrap();
  }
}

lazy_static! {
  /// Global config.
  pub static ref CONFIG: Cfg = Cfg::load(&ARGS.config_search_path);
}
