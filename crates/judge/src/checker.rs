use std::{collections::HashMap, str::FromStr, time};

use regex::Regex;
use thiserror::Error;

use crate::{
  etc, result,
  sandbox::{self, proto},
};

/// Parsed testlib checker output.
#[derive(Debug, PartialEq)]
pub struct Output {
  /// Testlib parsed status.
  pub status: result::Status,

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
        Regex::new(r"(?m)^[ \t]*(status|score)\((\w+)\)[ \t]*(.*?)\s*$").unwrap();
    }

    let mut ret = Self {
      status: result::Status::SystemError,
      score: 0.,
      message: result::limit_message(output),
    };

    if let Some(cap) = AC_PAT.captures(output) {
      ret = Self {
        status: result::Status::Accepted,
        score: 1.,
        message: result::limit_message(&format!("ac {}", &cap[1])),
      };
    } else if let Some(cap) = WA_PAT.captures(output) {
      ret = Self {
        status: result::Status::WrongAnswer,
        score: 0.,
        message: result::limit_message(&format!("wa {}", &cap[1])),
      };
    } else if let Some(cap) = FAIL_PAT.captures(output) {
      ret = Self {
        status: result::Status::SystemError,
        score: 0.,
        message: result::limit_message(&format!("fail {}", &cap[1])),
      };
    } else if let Some(cap) = PE_PAT.captures(output) {
      ret = Self {
        status: result::Status::PresentationError,
        score: 0.,
        message: result::limit_message(&format!("pe {}", &cap[1])),
      };
    } else if let Some(cap) = PC_PAT.captures(output) {
      if let Ok(score) = cap[1].parse::<f32>() {
        if score >= 1. {
          ret = Self {
            status: result::Status::Accepted,
            score: 1.,
            message: result::limit_message(&format!("ac {}", &cap[2])),
          };
        } else if score <= 0. {
          ret = Self {
            status: result::Status::WrongAnswer,
            score: 0.,
            message: result::limit_message(&format!("wa {}", &cap[2])),
          };
        } else {
          ret = Self {
            status: result::Status::PartiallyCorrect,
            score,
            message: result::limit_message(&format!("pc {}", &cap[2])),
          };
        }
      }
    }

    for cap in CUSTOM_PAT.captures_iter(output) {
      if &cap[1] == "status" {
        if let Ok(stat) = result::Status::from_str(&cap[2]) {
          ret.status = stat;
        }
      } else if &cap[1] == "score" {
        if let Ok(stat) = cap[2].parse::<f32>() {
          ret.score = stat.clamp(0., 1.);
        }
      }
    }

    return ret;
  }
}

/// Error when the checker behaves abnormally.
///
/// Such as being compile limit exceed or signaled.
#[derive(Debug, Error)]
pub enum Error {
  #[error(
    "checker runs failed (status: {status:?}, \
    time: {time:?}, memory: {memory} bytes, message: {message})"
  )]
  Execute {
    status: proto::StatusType,
    time: time::Duration,
    memory: u64,
    message: String,
    exit_code: i32,
  },

  #[error("sandbox error")]
  Sandbox(#[from] tonic::Status),
}

impl From<proto::Result> for Error {
  fn from(res: proto::Result) -> Self {
    return Self::Execute {
      status: res.status(),
      message: result::limit_message(&String::from_utf8_lossy(&res.files["stderr"])),
      memory: res.memory,
      time: time::Duration::from_nanos(res.time),
      exit_code: res.exit_status,
    };
  }
}

impl sandbox::Client {
  /// Run the checker with input, output and answer file.
  ///
  /// Returns the parsed testlib output.
  pub async fn check(
    &self,
    lang: &etc::LangCfg,
    args: Vec<String>,
    exec: proto::File,
    inf: proto::File,
    ouf: proto::File,
    ans: proto::File,
    mut copy_in: HashMap<String, proto::File>,
  ) -> Result<Output, Error> {
    copy_in.insert(lang.exec.clone(), exec);
    copy_in.insert("inf.txt".to_string(), inf);
    copy_in.insert("ouf.txt".to_string(), ouf);
    copy_in.insert("ans.txt".to_string(), ans);

    let cmd = proto::Cmd {
      args: [
        lang.run_cmd.clone(),
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
    };

    return match self.exec(vec![cmd], vec![]).await {
      Ok(res) => match res.results[0].status() {
        proto::StatusType::Accepted | proto::StatusType::NonZeroExitStatus => Ok(Output::parse(
          &String::from_utf8_lossy(&res.results[0].files["stderr"]),
        )),
        _ => Err(res.results[0].clone().into()),
      },
      Err(e) => Err(Error::Sandbox(e)),
    };
  }
}
