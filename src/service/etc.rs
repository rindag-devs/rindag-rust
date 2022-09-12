use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::RwLock};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cfg {
  // The address for the RinDAG http server to listen on
  pub addr: String,

  pub postgres: PostgresCfg,

  pub redis: RedisCfg,

  pub git: GitCfg,

  pub lang: HashMap<String, LangCfg>,
}

impl Default for Cfg {
  // Set default values for config
  fn default() -> Self {
    return Cfg {
      addr: ":8080".to_string(),
      postgres: PostgresCfg {
        host: "localhost".to_string(),
        port: 5432,
        user: "root".to_string(),
        password: "root".to_string(),
        db_name: "rindag".to_string(),
        use_ssl: false,
      },
      redis: RedisCfg {
        addr: "localhost:6379".to_string(),
        password: "".to_string(),
        db: 0,
      },
      git: GitCfg {
        exec_path: "/usr/bin/git".to_string(),
        repo_path: "/var/lib/rindag/git".to_string(),
      },
      lang: HashMap::from([
        (
          "c".to_string(),
          LangCfg {
            compile_cmd: ["/usr/bin/gcc", "foo.c", "-o", "foo", "-O2"]
              .iter()
              .map(|&s| s.into())
              .collect(),
            run_cmd: vec!["foo".to_string()],
            source_name: "foo.c".to_string(),
            exec_name: "foo".to_string(),
          },
        ),
        (
          "cpp".to_string(),
          LangCfg {
            compile_cmd: ["/usr/bin/g++", "foo.cpp", "-o", "foo", "-O2"]
              .iter()
              .map(|&s| s.into())
              .collect(),
            run_cmd: vec!["foo".to_string()],
            source_name: "foo.cpp".to_string(),
            exec_name: "foo".to_string(),
          },
        ),
      ]),
    };
  }
}

// Postgresql database config
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PostgresCfg {
  pub host: String,
  pub port: i32,
  pub user: String,
  pub password: String,
  pub db_name: String,
  pub use_ssl: bool,
}

// Redis config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedisCfg {
  pub addr: String,
  pub password: String,
  pub db: i32,
}

// Git config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitCfg {
  // Git exec path
  // Like `/usr/bin/git`
  pub exec_path: String,

  // Path to the git repositories
  // Like `/var/lib/rindag/git`
  pub repo_path: String,
}

// Problem build config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildCfg {
  // Path to storage the problem build files
  // Like `/var/lib/rindag/build`
  pub storage_path: String,

  // Auto build problem when push to master branch
  pub build_when_push: bool,
}

// Programming language config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LangCfg {
  pub compile_cmd: Vec<String>,

  pub run_cmd: Vec<String>,

  // Name of source file
  pub source_name: String,

  // Name of executable file
  pub exec_name: String,
}

lazy_static! {
  pub static ref CONFIG: RwLock<Cfg> = RwLock::new(Cfg::default());
}

pub fn load_config(search_paths: &Vec<String>) {
  let mut builder = config::Config::builder()
    .add_source(config::Config::try_from(&Cfg::default()).unwrap())
    .add_source(config::File::with_name("/etc/rindag/config").required(false));

  for p in search_paths {
    builder = builder.add_source(config::File::with_name(p.as_str()).required(false));
  }

  builder = builder.add_source(config::Environment::with_prefix("RINDAG"));

  *CONFIG.write().unwrap() = builder.build().unwrap().try_deserialize::<Cfg>().unwrap();
}
