use std::{collections::HashMap, str::FromStr};

use regex::Regex;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{program, result, sandbox};

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
      static ref PC_PAT: Regex =
        Regex::new(r"\A(?:partially correct|points) \(?([0-9]*\.?[0-9]*)\)?").unwrap();
      static ref CUSTOM_PAT: Regex =
        Regex::new(r"(?m)^[ \t]*(status|score)\(([\w\.]+)\)[ \t]*(.*?)\s*$").unwrap();
    }

    let mut ret = (Status::SystemError, 0.);

    if output.starts_with("ok") {
      ret = (Status::Accepted, 1.);
    } else if output.starts_with("wrong answer") {
      ret = (Status::WrongAnswer, 0.);
    } else if output.starts_with("FAIL") {
      ret = (Status::SystemError, 0.);
    } else if output.starts_with("wrong output format") {
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

/// Checker is a type of executable program,
/// which is used to check whether the answer obtained by the user's program is consistent with the
/// standard answer on a given input, or to judge the "correctness" of the user's answer.
#[derive(Debug, Clone)]
pub struct Checker {
  pub exec: program::Executable,
}

impl From<program::Executable> for Checker {
  fn from(exec: program::Executable) -> Self {
    Self { exec }
  }
}

impl Checker {
  /// Run the checker with input, output and answer file.
  ///
  /// Returns the parsed testlib output.
  pub async fn check(
    &self,
    args: Vec<String>,
    input_file: sandbox::FileHandle,
    output_file: sandbox::FileHandle,
    answer_file: sandbox::FileHandle,
    mut copy_in: HashMap<String, sandbox::FileHandle>,
  ) -> Result<Output, result::RuntimeError> {
    copy_in.insert(self.exec.lang.exec().to_string(), self.exec.file.clone());
    copy_in.insert("inf.txt".to_string(), input_file);
    copy_in.insert("ouf.txt".to_string(), output_file);
    copy_in.insert("ans.txt".to_string(), answer_file);

    let mut res = sandbox::Request::Run(sandbox::Cmd {
      args: [
        self.exec.lang.run_cmd().clone(),
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
    .await;

    assert_eq!(res.len(), 1);
    let res = res.pop().unwrap();

    match res.result.status {
      sandbox::Status::Accepted | sandbox::Status::NonZeroExitStatus => Ok(Output::parse(
        &String::from_utf8_lossy(&res.files["stderr"].context().await.unwrap()),
      )),
      _ => Err(res.result.into()),
    }
  }
}
