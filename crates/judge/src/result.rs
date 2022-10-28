use std::time;

use serde::{Deserialize, Serialize};
use strum::Display;
use thiserror::Error;

use crate::{checker, sandbox};

/// Limit the message to a maximum of 'LIMIT' characters.
pub fn limit_message(s: &str) -> String {
  const LIMIT: usize = 1024;
  if s.as_bytes().len() <= LIMIT {
    return s.to_string();
  }
  return String::from_utf8_lossy(&s.bytes().into_iter().take(LIMIT - 3).collect::<Vec<_>>())
    .to_string()
    + "...";
}

/// Error when task does not executed normally (result != Accepted).
#[derive(Debug, Error, Clone)]
#[error(
    "task executed failed (status: {0}, time: {1:?}, memory: {2} bytes)",
    result.status,result.time,result.memory,
  )]
pub struct RuntimeError {
  pub result: sandbox::ExecuteResult,
}

impl From<sandbox::ExecuteResult> for RuntimeError {
  fn from(res: sandbox::ExecuteResult) -> Self {
    Self { result: res }
  }
}

/// Judge result status for a program.
#[derive(Debug, PartialEq, strum::EnumString, Serialize, Deserialize, Clone, Display)]
#[strum(serialize_all = "snake_case")]
pub enum RecordStatus {
  Waiting,
  Skipped,
  Accepted,
  WrongAnswer,
  PartiallyCorrect,
  PresentationError,
  TimeLimitExceeded,
  MemoryLimitExceeded,
  OutputLimitExceeded,
  FileError,
  RuntimeError,
  SystemError,
}

impl From<sandbox::Status> for RecordStatus {
  fn from(s: sandbox::Status) -> Self {
    match s {
      sandbox::Status::Accepted => Self::Accepted,
      sandbox::Status::TimeLimitExceeded => Self::TimeLimitExceeded,
      sandbox::Status::MemoryLimitExceeded => Self::MemoryLimitExceeded,
      sandbox::Status::OutputLimitExceeded => Self::OutputLimitExceeded,
      sandbox::Status::FileError => Self::FileError,
      sandbox::Status::NonZeroExitStatus => Self::RuntimeError,
      sandbox::Status::DangerousSyscall => Self::RuntimeError,
      sandbox::Status::Signalled => Self::RuntimeError,
      sandbox::Status::InternalError => Self::SystemError,
    }
  }
}

impl From<checker::Status> for RecordStatus {
  fn from(s: checker::Status) -> Self {
    match s {
      checker::Status::Accepted => Self::Accepted,
      checker::Status::WrongAnswer => Self::WrongAnswer,
      checker::Status::PartiallyCorrect => Self::PartiallyCorrect,
      checker::Status::PresentationError => Self::PresentationError,
      checker::Status::SystemError => Self::SystemError,
    }
  }
}

/// A judge record of a solution running a single test.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Record {
  /// Judge status.
  pub status: RecordStatus,

  /// Code run time.
  pub time: time::Duration,

  /// Memory in bytes.
  pub memory: u64,

  /// Exit code.
  pub exit_code: i32,

  /// Score.
  pub score: f32,

  /// A message for human reading (like status explanation or checker message).
  pub message: String,
}

lazy_static! {
  pub static ref RECORD_WAITING: Record = Record {
    status: RecordStatus::Waiting,
    time: time::Duration::ZERO,
    memory: 0,
    exit_code: -1,
    score: 0.,
    message: "waiting".to_string(),
  };
  pub static ref RECORD_SKIPPED: Record = Record {
    status: RecordStatus::Skipped,
    time: time::Duration::ZERO,
    memory: 0,
    exit_code: -1,
    score: 0.,
    message: "skipped".to_string(),
  };
}

impl Record {
  /// Combine a JudgeResult and a checker::Output into a Record.
  pub fn new(result: sandbox::ExecuteResult, checker_output: Option<checker::Output>) -> Self {
    if checker_output.is_none() {
      if result.status != sandbox::Status::Accepted {
        return Self {
          status: RecordStatus::SystemError,
          time: result.time,
          memory: result.memory,
          exit_code: result.exit_code,
          score: 0.,
          message: "error: no checker".to_string(),
        };
      }
      return Self {
        status: result.status.clone().into(),
        time: result.time,
        memory: result.memory,
        exit_code: result.exit_code,
        score: 0.,
        message: RuntimeError::from(result).to_string(),
      };
    }
    let checker_output = checker_output.unwrap();
    Self {
      status: checker_output.status.into(),
      time: result.time,
      memory: result.memory,
      exit_code: result.exit_code,
      score: checker_output.score,
      message: checker_output.message,
    }
  }
}

/// Judgement result of an entire problem.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum JudgeResult {
  Ok {
    score: f32,
    results: Vec<Vec<Record>>,
  },
  CompileError {
    message: String,
  },
}

impl JudgeResult {
  pub fn from_compile_error(err: RuntimeError) -> Self {
    Self::CompileError {
      message: err.to_string(),
    }
  }
}
