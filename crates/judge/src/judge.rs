use std::{collections::HashMap, sync::Arc, time};

use crate::{program, sandbox};

impl program::Executable {
  /// Run the given executable file on a test case of batch problem (aka. traditional problem),
  /// and then returns the judgement result and the output file.
  ///
  /// Second return value =
  ///
  /// - JudgeResult == AC => Some(file id of stdout)
  /// - Otherwise => None
  pub async fn judge_batch(
    &self,
    args: Vec<String>,
    input_file: Arc<sandbox::FileHandle>,
    mut copy_in: HashMap<String, Arc<sandbox::FileHandle>>,
    time_limit: time::Duration,
    memory_limit: u64,
  ) -> (sandbox::ExecuteResult, Option<Arc<sandbox::FileHandle>>) {
    copy_in.insert(self.lang.exec().to_string(), self.file.clone());

    let mut res = sandbox::Request::Run(sandbox::Cmd {
      args: [self.lang.run_cmd().clone(), args].concat(),
      stdin: Some(input_file),
      copy_in,
      copy_out: vec!["stdout".to_string(), "stderr".to_string()],
      time_limit,
      memory_limit,
      ..Default::default()
    })
    .exec()
    .await;

    assert_eq!(res.len(), 1);
    let res = res.pop().unwrap();

    (
      res.result.clone(),
      match res.result.status {
        sandbox::Status::Accepted => Some(res.files["stdout"].clone()),
        _ => None,
      },
    )
  }
}
