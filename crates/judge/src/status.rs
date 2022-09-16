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
