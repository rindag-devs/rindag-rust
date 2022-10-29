use std::{collections::HashMap, str::FromStr, sync::Arc};

use regex::Regex;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{etc, result, sandbox};

#[derive(Debug, PartialEq, strum::EnumString, Serialize, Deserialize, Clone, Display)]
#[strum(serialize_all = "snake_case")]
pub enum Status {
  Accepted,
  WrongAnswer,
  PartiallyCorrect,
  PresentationError,
  SystemError,
}

/// Parsed testlib checker output.
#[derive(Debug, PartialEq, Clone)]
pub struct Output {
  /// Testlib parsed status.
  pub status: Status,

  /// Length limited output message.
  pub message: String,

  /// Floating point score value in [0,1].
  pub score: f32,
}

impl Output {
  /// Parse the output of testlib checker.
  ///
  /// - ok -> Accepted.
  /// - wrong answer -> WrongAnswer.
  /// - fail -> SystemError.
  /// - wrong output format -> PresentationError.
  /// - points or partially correct:
  ///   - score <= 0: WrongAnswer, real_score = 0.
  ///   - 0 < score < 1: PartiallyCorrect, real_score = score.
  ///   - score >= 1: Accepted, real_score = 1.
  ///
  /// If there is a line in the output that starts with `status(...)`,
  /// it will try to use the value in parentheses as the result status.
  ///
  /// If there is a line in the output that starts with `score(...)`,
  /// it will try to use the number in parentheses as the result score.
  pub fn parse(output: &str) -> Self {
    lazy_static! {
      static ref AC_PAT: Regex = Regex::new(r"(?s)\Aok\s*(.*?)\s*\z").unwrap();
      static ref WA_PAT: Regex = Regex::new(r"(?s)\Awrong answer\s*(.*?)\s*\z").unwrap();
      static ref FAIL_PAT: Regex = Regex::new(r"(?s)\AFAIL\s*(.*?)\s*\z").unwrap();
      static ref PE_PAT: Regex = Regex::new(r"(?s)\Awrong output format\s*(.*?)\s*\z").unwrap();
      static ref PC_PAT: Regex =
        Regex::new(r"(?s)\A(?:partially correct|points) \(?([0-9]*\.?[0-9]*)\)?\s*(.*?)\s*\z")
          .unwrap();
      static ref CUSTOM_PAT: Regex =
        Regex::new(r"(?m)^[ \t]*(status|score)\(([\w\.]+)\)[ \t]*(.*?)\s*$").unwrap();
    }

    let mut ret = (Status::SystemError, 0.);

    if AC_PAT.is_match(output) {
      ret = (Status::Accepted, 1.);
    } else if WA_PAT.is_match(output) {
      ret = (Status::WrongAnswer, 0.);
    } else if FAIL_PAT.is_match(output) {
      ret = (Status::SystemError, 0.);
    } else if PE_PAT.is_match(output) {
      ret = (Status::PresentationError, 0.);
    } else if let Some(cap) = PC_PAT.captures(output) {
      if let Ok(score) = cap[1].parse::<f32>() {
        if score >= 1. {
          ret = (Status::Accepted, 1.);
        } else if score <= 0. {
          ret = (Status::WrongAnswer, 0.);
        } else {
          ret = (Status::PartiallyCorrect, score);
        }
      }
    }

    for cap in CUSTOM_PAT.captures_iter(output) {
      if &cap[1] == "status" {
        if let Ok(stat) = Status::from_str(&cap[2]) {
          ret.0 = stat;
        }
      } else if &cap[1] == "score" {
        if let Ok(stat) = cap[2].parse::<f32>() {
          ret.1 = stat.clamp(0., 1.);
        }
      }
    }

    return Self {
      status: ret.0,
      score: ret.1,
      message: result::limit_message(output),
    };
  }
}

/// Run the checker with input, output and answer file.
///
/// Returns the parsed testlib output.
pub async fn check(
  lang: &etc::LangCfg,
  args: Vec<String>,
  exec: Arc<sandbox::FileHandle>,
  inf: Arc<sandbox::FileHandle>,
  ouf: Arc<sandbox::FileHandle>,
  ans: Arc<sandbox::FileHandle>,
  mut copy_in: HashMap<String, Arc<sandbox::FileHandle>>,
) -> Result<Output, result::RuntimeError> {
  copy_in.insert(lang.exec().to_string(), exec);
  copy_in.insert("inf.txt".to_string(), inf);
  copy_in.insert("ouf.txt".to_string(), ouf);
  copy_in.insert("ans.txt".to_string(), ans);

  let res = sandbox::Request::Run(sandbox::Cmd {
    args: [
      lang.run_cmd().clone(),
      vec![
        "inf.txt".to_string(),
        "ouf.txt".to_string(),
        "ans.txt".to_string(),
      ],
      args,
    ]
    .concat(),
    copy_in,
    copy_out: vec!["stderr".to_string()],
    ..Default::default()
  })
  .exec()
  .await[0]
    .clone();

  match res.result.status {
    sandbox::Status::Accepted | sandbox::Status::NonZeroExitStatus => Ok(Output::parse(
      &String::from_utf8_lossy(&res.files["stderr"].to_vec().await.unwrap()),
    )),
    _ => Err(res.result.into()),
  }
}
