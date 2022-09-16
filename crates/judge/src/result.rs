use crate::sandbox::exec;

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
  RuntimeError,
  CompileError,
  PresentationError,
  SystemError,
  Canceled,
  Skipped,
}

/// Compile result for a code.
pub struct CompileResult {
  pub status: exec::Status,
  pub stderr: String,
  pub stdout: String,
}
