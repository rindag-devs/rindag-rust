use std::str::FromStr;

use regex::Regex;

use crate::Status;

/// Testlib source code.
pub static TESTLIB_SOURCE: &str = include_str!("../third_party/testlib/testlib.h");

fn limit_str(s: &str) -> String {
  const LIMIT: usize = 1024;
  if s.len() <= LIMIT {
    return s.to_string();
  }
  return s.chars().take(LIMIT - 3).collect::<String>() + "...";
}

/// Parse the output of testlib.
///
/// If `fail_as_wa` is true,
/// the output starting with `FAIL` will be treated as `Status::WrongAnswer`,
/// otherwise it will be treated as `Status::SystemError`.
pub fn parse_output(output: &str, fail_as_wa: bool) -> (Status, f32, String) {
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

  let mut ret = (Status::SystemError, 0., limit_str(output));

  if let Some(cap) = AC_PAT.captures(output) {
    ret = (Status::Accepted, 1., limit_str(&format!("ac {}", &cap[1])));
  } else if let Some(cap) = WA_PAT.captures(output) {
    ret = (
      Status::WrongAnswer,
      0.,
      limit_str(&format!("wa {}", &cap[1])),
    );
  } else if let Some(cap) = FAIL_PAT.captures(output) {
    ret = (
      if fail_as_wa {
        Status::WrongAnswer
      } else {
        Status::SystemError
      },
      0.,
      limit_str(&format!("fail {}", &cap[1])),
    );
  } else if let Some(cap) = PE_PAT.captures(output) {
    ret = (
      Status::PresentationError,
      0.,
      limit_str(&format!("pe {}", &cap[1])),
    );
  } else if let Some(cap) = PC_PAT.captures(output) {
    if let Ok(score) = cap[1].parse::<f32>() {
      if score >= 1. {
        ret = (Status::Accepted, 1., limit_str(&format!("ac {}", &cap[2])));
      } else if score <= 0. {
        ret = (
          Status::WrongAnswer,
          0.,
          limit_str(&format!("wa {}", &cap[2])),
        );
      } else {
        ret = (
          Status::PartiallyCorrect,
          score,
          limit_str(&format!("pc {}", &cap[2])),
        );
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

  return ret;
}
