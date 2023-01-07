use thiserror::Error;

use crate::sandbox;

/// Error when task does not executed normally (result != Accepted).
#[derive(Debug, Error, Clone)]
#[error(
    "task executed failed (status: {0}, time: {1:?}, memory: {2} bytes, exit code: {3})",
    result.status, result.time, result.memory, result.exit_code
  )]
pub struct RuntimeError {
  pub result: sandbox::ExecuteResult,
}

impl From<sandbox::ExecuteResult> for RuntimeError {
  fn from(result: sandbox::ExecuteResult) -> Self {
    Self { result }
  }
}

/// Error when program does not compile successful.
#[derive(Debug, Error, Clone)]
#[error(
    "compile failed (status: {0}, time: {1:?}, memory: {2} bytes, exit code: {3}): {message}",
    result.status, result.time, result.memory, result.exit_code
  )]
pub struct CompileError {
  pub result: sandbox::ExecuteResult,

  /// Compile message, usually the error message output by the compiler.
  pub message: String,
}
