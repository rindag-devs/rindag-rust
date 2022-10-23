use std::time;

use serde::{Deserialize, Serialize};
use strum::{Display, IntoEnumIterator};
use thiserror::Error;

use crate::{checker, sandbox::proto};

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

#[derive(Debug, PartialEq, strum::EnumString, strum::Display, strum::EnumIter)]
#[strum(serialize_all = "snake_case")]
pub enum Signal {
  Hangup = 1,
  Interrupt = 2,
  Quit = 3,
  IllegalInstruction = 4,
  TraceBreakpointTrap = 5,
  Aborted = 6,
  BusError = 7,
  FloatingPointException = 8,
  Killed = 9,
  UserDefinedSignal1 = 10,
  SegmentationFault = 11,
  UserDefinedSignal2 = 12,
  BrokenPipe = 13,
  AlarmClock = 14,
  Terminated = 15,
  StackFault = 16,
  ChildExited = 17,
  Continued = 18,
  StoppedSignal = 19,
  Stopped = 20,
  StoppedTtyInput = 21,
  StoppedTtyOutput = 22,
  UrgentIOCondition = 23,
  CPUTimeLimitExceeded = 24,
  FileSizeLimitExceeded = 25,
  VirtualTimerExpired = 26,
  ProfilingTimerExpired = 27,
  WindowChanged = 28,
  IOPossible = 29,
  PowerFailure = 30,
  BadSystemCall = 31,
}

/// Judge result status for a program.
/// This enum is only used to represent the result after executing the program,
/// and does not represent the result after the checker checks the correctness of the answer.
#[derive(Debug, PartialEq, strum::EnumString, Serialize, Deserialize, Clone, Display)]
#[strum(serialize_all = "snake_case")]
pub enum ExecuteStatus {
  Accepted,
  TimeLimitExceeded,
  MemoryLimitExceeded,
  OutputLimitExceeded,
  FileError,
  RuntimeError,
  SystemError,
}

impl From<proto::StatusType> for ExecuteStatus {
  fn from(s: proto::StatusType) -> Self {
    return match s {
      proto::StatusType::Invalid => Self::SystemError,
      proto::StatusType::Accepted => Self::Accepted,
      proto::StatusType::MemoryLimitExceeded => Self::MemoryLimitExceeded,
      proto::StatusType::TimeLimitExceeded => Self::TimeLimitExceeded,
      proto::StatusType::OutputLimitExceeded => Self::OutputLimitExceeded,
      proto::StatusType::FileError => Self::FileError,
      proto::StatusType::NonZeroExitStatus => Self::RuntimeError,
      proto::StatusType::Signalled => Self::RuntimeError,
      proto::StatusType::DangerousSyscall => Self::RuntimeError,
      proto::StatusType::InternalError => Self::SystemError,
    };
  }
}

/// Result of a program running on a single task.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecuteResult {
  /// Judge status.
  pub status: ExecuteStatus,

  /// Code run time.
  pub time: time::Duration,

  /// Memory in bytes.
  pub memory: u64,

  /// A cut prefix of stderr.
  pub stderr: String,

  /// Exit code.
  pub exit_code: i32,
}

impl From<proto::Result> for ExecuteResult {
  fn from(res: proto::Result) -> Self {
    Self {
      status: res.status().into(),
      time: time::Duration::from_nanos(res.time),
      memory: res.memory,
      exit_code: res.exit_status,
      stderr: match res.status() {
        proto::StatusType::Signalled => {
          format!(
            "signalled: {}",
            Signal::iter().nth(res.exit_status as usize).unwrap()
          )
        }
        proto::StatusType::NonZeroExitStatus => {
          format!("non_zero_exit_status: {}", res.exit_status)
        }
        proto::StatusType::InternalError => res.error.clone(),
        _ => limit_message(&String::from_utf8_lossy(&res.files["stderr"])),
      },
    }
  }
}

impl From<tonic::Status> for ExecuteResult {
  fn from(_: tonic::Status) -> Self {
    Self {
      status: ExecuteStatus::SystemError,
      time: time::Duration::ZERO,
      memory: 0,
      stderr: String::new(),
      exit_code: -1,
    }
  }
}

/// Error when task does not executed normally (result != Accepted).
#[derive(Debug, Error, Clone)]
pub enum Error {
  #[error(
    "task executed failed (status: {status}, \
    time: {time:?}, memory: {memory} bytes, stderr: {stderr})"
  )]
  Execute {
    status: ExecuteStatus,
    time: time::Duration,
    memory: u64,
    stderr: String,
    exit_code: i32,
  },

  #[error("sandbox error")]
  Sandbox(#[from] tonic::Status),
}

impl From<proto::Result> for Error {
  fn from(res: proto::Result) -> Self {
    return Self::Execute {
      status: res.status().into(),
      stderr: limit_message(&String::from_utf8_lossy(&res.files["stderr"])),
      memory: res.memory,
      time: time::Duration::from_nanos(res.time),
      exit_code: res.exit_status,
    };
  }
}

impl From<ExecuteResult> for Error {
  fn from(res: ExecuteResult) -> Self {
    return Self::Execute {
      status: res.status,
      stderr: res.stderr,
      memory: res.memory,
      time: res.time,
      exit_code: res.exit_code,
    };
  }
}

/// Judge result status for a program.
/// This enum is only used to represent the result after executing the program,
/// and does not represent the result after the checker checks the correctness of the answer.
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

impl From<ExecuteStatus> for RecordStatus {
  fn from(s: ExecuteStatus) -> Self {
    match s {
      ExecuteStatus::Accepted => Self::Accepted,
      ExecuteStatus::TimeLimitExceeded => Self::TimeLimitExceeded,
      ExecuteStatus::MemoryLimitExceeded => Self::MemoryLimitExceeded,
      ExecuteStatus::OutputLimitExceeded => Self::OutputLimitExceeded,
      ExecuteStatus::FileError => Self::FileError,
      ExecuteStatus::RuntimeError => Self::RuntimeError,
      ExecuteStatus::SystemError => Self::SystemError,
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

  /// A cut prefix of stderr.
  pub stderr: String,

  /// Exit code.
  pub exit_code: i32,

  /// Score.
  pub score: f32,

  /// Checker message.
  pub checker_message: String,
}

lazy_static! {
  pub static ref RECORD_WAITING: Record = Record {
    status: RecordStatus::Waiting,
    time: time::Duration::ZERO,
    memory: 0,
    stderr: "waiting".to_string(),
    exit_code: -1,
    score: 0.,
    checker_message: String::new(),
  };
  pub static ref RECORD_SKIPPED: Record = Record {
    status: RecordStatus::Skipped,
    time: time::Duration::ZERO,
    memory: 0,
    stderr: "skipped".to_string(),
    exit_code: -1,
    score: 0.,
    checker_message: String::new(),
  };
}

impl Record {
  /// Combine a JudgeResult and a checker::Output into a Record.
  pub fn new(result: ExecuteResult, checker_output: Option<checker::Output>) -> Self {
    if checker_output.is_none() {
      if result.status != ExecuteStatus::Accepted {
        return Self {
          status: RecordStatus::SystemError,
          time: result.time,
          memory: result.memory,
          stderr: result.stderr,
          exit_code: result.exit_code,
          score: 0.,
          checker_message: "error: no checker".to_string(),
        };
      }
      return Self {
        status: result.status.into(),
        time: result.time,
        memory: result.memory,
        stderr: result.stderr,
        exit_code: result.exit_code,
        score: 0.,
        checker_message: String::new(),
      };
    }
    let checker_output = checker_output.unwrap();
    return Self {
      status: checker_output.status.into(),
      time: result.time,
      memory: result.memory,
      stderr: result.stderr,
      exit_code: result.exit_code,
      score: checker_output.score,
      checker_message: checker_output.message,
    };
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
  SystemError {
    message: String,
  },
}

impl From<tonic::Status> for JudgeResult {
  fn from(err: tonic::Status) -> Self {
    Self::SystemError {
      message: err.to_string(),
    }
  }
}

impl JudgeResult {
  pub fn from_compile_error(err: Error) -> Self {
    if let Error::Sandbox(err) = err {
      return Self::SystemError {
        message: err.to_string(),
      };
    }
    Self::CompileError {
      message: err.to_string(),
    }
  }
}
