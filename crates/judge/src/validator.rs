use std::{collections::HashMap, sync::Arc};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{etc, result, sandbox};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VariableBounds {
  pub hit_min: bool,
  pub hit_max: bool,
}

// Parsed testlib validator overview.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Overview {
  pub variables: HashMap<String, VariableBounds>,
  pub features: HashMap<String, bool>,
}

impl Overview {
  /// Parse the overview log of testlib validator.
  pub fn parse(s: &str) -> Self {
    lazy_static! {
      static ref VAR_PAT: Regex =
        Regex::new("(?m-s)^\"(.*)\":(| min-value-hit)(| max-value-hit)$").unwrap();
      static ref FEA_PAT: Regex = Regex::new("(?m-s)^feature \"(.*)\":(| hit)$").unwrap();
    }

    let mut variables = HashMap::new();
    let mut features = HashMap::new();

    for cap in VAR_PAT.captures_iter(s) {
      variables.insert(
        cap[1].to_string(),
        VariableBounds {
          hit_min: !cap[2].is_empty(),
          hit_max: !cap[3].is_empty(),
        },
      );
    }

    for cap in FEA_PAT.captures_iter(s) {
      features.insert(cap[1].to_string(), !cap[2].is_empty());
    }

    return Self {
      variables,
      features,
    };
  }
}

/// Run the validator and returns the overview log file.
///
/// It will do these following:
///
/// 1. Constructs a sandbox request according to the validator language.
/// 2. Execute this request with sandbox.
/// 3. Check if there's an error happens, or return the parsed overview log.
///
/// # Errors
///
/// This function will return an error if validating abnormally
/// (e.g. validating time limit exceed or signaled)
/// or a sandbox internal error was encountered.
pub async fn validate(
  lang: &etc::LangCfg,
  args: Vec<String>,
  exec: Arc<sandbox::FileHandle>,
  inf: Arc<sandbox::FileHandle>,
  mut copy_in: HashMap<String, Arc<sandbox::FileHandle>>,
) -> Result<Overview, result::RuntimeError> {
  copy_in.insert(lang.exec.clone(), exec);

  let res = sandbox::Request::Run(sandbox::Cmd {
    args: [
      lang.run_cmd.clone(),
      args,
      [
        "--testOverviewLogFileName".to_string(),
        "val.log".to_string(),
      ]
      .to_vec(),
    ]
    .concat(),
    stdin: Some(inf),
    copy_in,
    copy_out: vec!["stderr".to_string(), "val.log".to_string()],
    ..Default::default()
  })
  .exec()
  .await[0]
    .clone();

  match res.result.status {
    sandbox::Status::Accepted => Ok(Overview::parse(&String::from_utf8_lossy(
      &res.files["val.log"].clone().to_vec().await.unwrap(),
    ))),
    _ => Err(res.result.into()),
  }
}
