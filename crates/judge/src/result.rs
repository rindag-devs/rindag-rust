use std::time;

use strum::IntoEnumIterator;

use crate::sandbox::proto;

/// Limit the message to a maximum of 'LIMIT' characters.
pub fn limit_message(s: &str) -> String {
  const LIMIT: usize = 1024;
  if s.len() <= LIMIT {
    return s.to_string();
  }
  return s.chars().take(LIMIT - 3).collect::<String>() + "...";
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
#[derive(Debug, PartialEq, strum::EnumString)]
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

/// Compile result for a code.
#[derive(Debug)]
pub struct CompileResult {
  pub status: proto::StatusType,
  pub stderr: String,
  pub stdout: String,
}

impl From<&proto::Result> for CompileResult {
  fn from(res: &proto::Result) -> Self {
    return Self {
      status: res.status(),
      stderr: limit_message(&String::from_utf8_lossy(&res.files["stderr"])),
      stdout: limit_message(&String::from_utf8_lossy(&res.files["stdout"])),
    };
  }
}

/// Judge result of a single test case.
#[derive(Debug)]
pub struct JudgeResult {
  /// Judge status.
  pub status: Status,

  /// Code run time.
  pub time: time::Duration,

  /// Memory in bytes.
  pub memory: u64,

  /// A cut prefix of stderr.
  pub stderr: String,
}

impl From<&proto::Result> for JudgeResult {
  fn from(res: &proto::Result) -> Self {
    return Self {
      status: res.status().into(),
      time: time::Duration::from_nanos(res.time),
      memory: res.memory,
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
