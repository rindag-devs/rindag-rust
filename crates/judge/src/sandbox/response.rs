use std::{collections::HashMap, time};

use serde::{Deserialize, Serialize};
use strum::Display;
use thiserror::Error;

use super::{proto, FileHandle};

/// Judging result of one `Cmd`, contains the execution result and the copy out files.
#[derive(Debug, Clone)]
pub struct ResponseResult {
  pub result: ExecuteResult,
  pub files: HashMap<String, FileHandle>,
}

/// Execution result of one `Cmd`.
#[derive(Debug, Clone)]
pub struct ExecuteResult {
  pub status: Status,
  pub time: time::Duration,
  pub memory: u64,
  pub exit_code: i32,
}

/// Judge result status for a program.
/// This enum is only used to represent the result after executing the program,
/// and does not represent the result after the checker checks the correctness of the answer.
#[derive(Debug, PartialEq, strum::EnumString, Serialize, Deserialize, Clone, Display)]
#[strum(serialize_all = "snake_case")]
pub enum Status {
  Accepted,
  TimeLimitExceeded,
  MemoryLimitExceeded,
  OutputLimitExceeded,
  FileError,
  NonZeroExitStatus,
  DangerousSyscall,
  Signalled,
  InternalError,
}

impl From<proto::response::result::StatusType> for Status {
  fn from(s: proto::response::result::StatusType) -> Self {
    match s {
      proto::response::result::StatusType::Invalid => Status::InternalError,
      proto::response::result::StatusType::Accepted => Status::Accepted,
      proto::response::result::StatusType::MemoryLimitExceeded => Status::MemoryLimitExceeded,
      proto::response::result::StatusType::TimeLimitExceeded => Status::TimeLimitExceeded,
      proto::response::result::StatusType::OutputLimitExceeded => Status::OutputLimitExceeded,
      proto::response::result::StatusType::FileError => Status::FileError,
      proto::response::result::StatusType::NonZeroExitStatus => Status::NonZeroExitStatus,
      proto::response::result::StatusType::Signalled => Status::Signalled,
      proto::response::result::StatusType::DangerousSyscall => Status::DangerousSyscall,
      proto::response::result::StatusType::InternalError => Status::InternalError,
    }
  }
}

#[derive(Debug, Clone, Error)]
#[error("sandbox error: {message}")]
pub struct SandboxError {
  pub message: String,
}

impl From<proto::response::Result> for ResponseResult {
  fn from(res: proto::response::Result) -> Self {
    Self {
      result: ExecuteResult {
        status: res.status().into(),
        time: time::Duration::from_nanos(res.time),
        memory: res.memory,
        exit_code: res.exit_status,
      },
      files: res
        .file_ids
        .into_iter()
        .map(|f| (f.0, FileHandle::from_id(f.1)))
        .collect(),
    }
  }
}
