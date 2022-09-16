use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, sync::RwLock, time};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
/// RinDAG server config.
pub struct Cfg {
  /// The address for the RinDAG http server to listen on.
  pub host: String,

  /// Judge token secret.
  ///
  /// Set to `None` to disable auth.
  ///
  /// WARNING: Be sure to set a token secret in a production environment.
  pub secret: Option<String>,

  pub lang: HashMap<String, LangCfg>,

  pub sandbox: SandboxCfg,
}

impl Default for Cfg {
  // Set default values for config
  fn default() -> Self {
    return Cfg {
      host: ":8080".to_string(),
      secret: None,
      lang: HashMap::from([
        (
          "c".to_string(),
          LangCfg {
            compile_cmd: [
              "/usr/bin/gcc",
              "foo.c",
              "-o",
              "foo",
              "-O2",
              "-w",
              "-fmax-errors=3",
            ]
            .iter()
            .map(|&s| s.into())
            .collect(),
            run_cmd: vec!["foo".to_string()],
            source: "foo.c".to_string(),
            exec: "foo".to_string(),
          },
        ),
        (
          "cpp".to_string(),
          LangCfg {
            compile_cmd: [
              "/usr/bin/g++",
              "foo.cpp",
              "-o",
              "foo",
              "-O2",
              "-w",
              "-fmax-errors=3",
            ]
            .iter()
            .map(|&s| s.into())
            .collect(),
            run_cmd: vec!["foo".to_string()],
            source: "foo.cpp".to_string(),
            exec: "foo".to_string(),
          },
        ),
      ]),
      sandbox: SandboxCfg {
        env: vec![
          "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
          "HOME=/w".to_string(),
        ],
        time_limit: time::Duration::from_secs(10),
        memory_limit: 1024 * 1024 * 1024, // 1 GB
        process_limit: 16,                // 16 processes
        stdout_limit: 512 * 1024 * 1024,  // 512 MB
        stderr_limit: 16 * 1024,          // 16 kB
        http_host: url::Url::from_str("http://localhost:5050").unwrap(),
        ws_host: url::Url::from_str("ws://localhost:5050/ws").unwrap(),
      },
    };
  }
}

/// Programming language config.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LangCfg {
  pub compile_cmd: Vec<String>,

  pub run_cmd: Vec<String>,

  /// Name of source file
  pub source: String,

  /// Name of executable file
  pub exec: String,
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
  pub stdout_limit: u64,

  /// Default stderr limit, in bytes.
  pub stderr_limit: u64,

  /// Sandbox http host.
  pub http_host: url::Url,

  /// Sandbox websocket host.
  pub ws_host: url::Url,
}

lazy_static! {
  /// Global config.
  pub static ref CONFIG: RwLock<Cfg> = RwLock::new(Cfg::default());
}

/// Load the global config.
///
/// It should be called on the top of `main` fn.
pub fn load_config(search_paths: &Vec<String>) {
  let mut builder = config::Config::builder()
    .add_source(config::File::with_name("/etc/rindag/judge").required(false));

  for p in search_paths {
    builder = builder.add_source(config::File::with_name(p.as_str()).required(false));
  }

  builder = builder.add_source(config::Environment::with_prefix("RINDAG_JUDGE"));

  *CONFIG.write().unwrap() = builder.build().unwrap().try_deserialize::<Cfg>().unwrap();
}
