use std::str::FromStr;

use regex::Regex;

use crate::result;

/// Testlib source code.
pub static TESTLIB_SOURCE: &str = include_str!("../third_party/testlib/testlib.h");

/// Parse the output of testlib.
///
/// If `fail_as_wa` is true,
/// the output starting with `FAIL` will be treated as `Status::WrongAnswer`,
/// otherwise it will be treated as `Status::SystemError`.
pub fn parse_output(output: &str, fail_as_wa: bool) -> (result::Status, f32, String) {
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

  let mut ret = (
    result::Status::SystemError,
    0.,
    result::limit_message(output),
  );

  if let Some(cap) = AC_PAT.captures(output) {
    ret = (
      result::Status::Accepted,
      1.,
      result::limit_message(&format!("ac {}", &cap[1])),
    );
  } else if let Some(cap) = WA_PAT.captures(output) {
    ret = (
      result::Status::WrongAnswer,
      0.,
      result::limit_message(&format!("wa {}", &cap[1])),
    );
  } else if let Some(cap) = FAIL_PAT.captures(output) {
    ret = (
      if fail_as_wa {
        result::Status::WrongAnswer
      } else {
        result::Status::SystemError
      },
      0.,
      result::limit_message(&format!("fail {}", &cap[1])),
    );
  } else if let Some(cap) = PE_PAT.captures(output) {
    ret = (
      result::Status::PresentationError,
      0.,
      result::limit_message(&format!("pe {}", &cap[1])),
    );
  } else if let Some(cap) = PC_PAT.captures(output) {
    if let Ok(score) = cap[1].parse::<f32>() {
      if score >= 1. {
        ret = (
          result::Status::Accepted,
          1.,
          result::limit_message(&format!("ac {}", &cap[2])),
        );
      } else if score <= 0. {
        ret = (
          result::Status::WrongAnswer,
          0.,
          result::limit_message(&format!("wa {}", &cap[2])),
        );
      } else {
        ret = (
          result::Status::PartiallyCorrect,
          score,
          result::limit_message(&format!("pc {}", &cap[2])),
        );
      }
    }
  }

  for cap in CUSTOM_PAT.captures_iter(output) {
    if &cap[1] == "status" {
      if let Ok(stat) = result::Status::from_str(&cap[2]) {
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
