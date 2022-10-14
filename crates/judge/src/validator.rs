use std::{collections::HashMap, sync::Arc};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
  etc, result,
  sandbox::{self, proto},
  CONFIG,
};

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

impl sandbox::Client {
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
    &self,
    lang: &etc::LangCfg,
    args: Vec<String>,
    exec: proto::File,
    inf: proto::File,
    mut copy_in: HashMap<String, proto::File>,
  ) -> Result<Overview, result::Error> {
    let c = &CONFIG.sandbox;

    copy_in.insert(lang.exec.clone(), exec);

    let cmd = proto::Cmd {
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
      files: vec![
        inf,
        proto::File::Pipe(proto::PipeCollector {
          name: "stdout".to_string(),
          max: c.stdout_limit,
          pipe: false,
        }),
        proto::File::Pipe(proto::PipeCollector {
          name: "stderr".to_string(),
          max: c.stderr_limit,
          pipe: false,
        }),
      ],
      copy_in,
      copy_out: vec!["stderr".to_string(), "val.log".to_string()],
      ..Default::default()
    };

    return match self.exec(vec![cmd], vec![]).await {
      Ok(res) => match res.results[0].status() {
        proto::StatusType::Accepted => Ok(Overview::parse(&String::from_utf8_lossy(
          &res.results[0].files["val.log"],
        ))),
        _ => Err(res.results[0].clone().into()),
      },
      Err(e) => Err(result::Error::Sandbox(Arc::new(e))),
    };
  }
}
