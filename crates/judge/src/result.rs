use std::{sync::Arc, time};

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use thiserror::Error;

use crate::sandbox::proto;

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

/// Judge result status on a single test case.
#[derive(Debug, PartialEq, strum::EnumString, Serialize, Deserialize, Clone)]
#[strum(serialize_all = "snake_case")]
pub enum Status {
  Waiting,
  Judging,
  Accepted,
  WrongAnswer,
  PartiallyCorrect,
  TimeLimitExceeded,
  MemoryLimitExceeded,
  OutputLimitExceeded,
  CompileError, // Only be used in `Record`.
  FileError,
  PresentationError,
  RuntimeError,
  SystemError,
  Canceled,
  Skipped,
}

impl From<proto::StatusType> for Status {
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

/// Result of a single task.
#[derive(Debug, Serialize, Deserialize)]
pub struct JudgeResult {
  /// Judge status.
  pub status: Status,

  /// Code run time.
  pub time: time::Duration,

  /// Memory in bytes.
  pub memory: u64,

  /// A cut prefix of stderr.
  pub stderr: String,

  /// Exit code.
  pub exit_code: i32,
}

impl From<proto::Result> for JudgeResult {
  fn from(res: proto::Result) -> Self {
    return Self {
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
    };
  }
}

// Error when task does not executed normally (result != Accepted).
#[derive(Debug, Error, Clone)]
pub enum Error {
  #[error(
    "task executed failed (status: {status:?}, \
    time: {time:?}, memory: {memory} bytes, stderr: {stderr})"
  )]
  Execute {
    status: Status,
    time: time::Duration,
    memory: u64,
    stderr: String,
    exit_code: i32,
  },

  #[error("sandbox error")]
  Sandbox(#[from] Arc<tonic::Status>),
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

impl From<JudgeResult> for Error {
  fn from(res: JudgeResult) -> Self {
    return Self::Execute {
      status: res.status,
      stderr: res.stderr,
      memory: res.memory,
      time: res.time,
      exit_code: res.exit_code,
    };
  }
}
