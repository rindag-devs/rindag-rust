use std::collections::HashMap;

use crate::{error, program, sandbox};

/// Answer of test case.
#[derive(Debug, Clone)]
pub enum Answer {
  /// The answer file is generated according to the main correct solution of the problem.
  Generated,

  /// Use plain text as answer file.
  Plain { context: Vec<u8> },
}

impl Answer {
  /// Make the input and upload to sandbox.
  pub async fn make(
    &self,
    standard_solution: &program::Executable,
    input_file: sandbox::FileHandle,
    copy_in: HashMap<String, sandbox::FileHandle>,
    time_limit: std::time::Duration,
    memory_limit: u64,
  ) -> Result<sandbox::FileHandle, error::RuntimeError> {
    match self {
      Answer::Generated => {
        let (res, file) = standard_solution
          .judge_batch(vec![], input_file, copy_in, time_limit, memory_limit)
          .await;
        if res.status != sandbox::Status::Accepted {
          return Err(error::RuntimeError::from(res));
        }
        Ok(file.unwrap())
      }
      Answer::Plain { context } => Ok(sandbox::FileHandle::upload(context).await),
    }
  }
}
