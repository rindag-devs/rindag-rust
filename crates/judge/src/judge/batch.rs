use std::{collections::HashMap, time};

use crate::{
  etc, result,
  sandbox::{self, proto},
  CONFIG,
};

impl sandbox::Client {
  /// Run the given executable file on a test case of batch problem (aka. traditional problem),
  /// and then returns the judgement result and the output file.
  ///
  /// Second return value =
  ///
  /// - JudgeResult == AC => Some(file id of stdout)
  /// - Otherwise => None
  pub async fn judge_batch(
    &self,
    lang: &etc::LangCfg,
    args: Vec<String>,
    exec: proto::File,
    inf: proto::File,
    mut copy_in: HashMap<String, proto::File>,
    time_limit: time::Duration,
    memory_limit: u64,
  ) -> (result::JudgeResult, Option<String>) {
    let c = &CONFIG.sandbox;

    copy_in.insert(lang.exec.clone(), exec);

    let cmd = proto::Cmd {
      args: [lang.run_cmd.clone(), args].concat(),
      files: vec![
        inf,
        proto::File::Pipe(proto::PipeCollector {
          name: "stdout".to_string(),
          max: c.stdout_limit,
          pipe: false,
        }),
        proto::File::Pipe(proto::PipeCollector {
          name: "stderr".to_string(),
          max: c.stderr_limit,
          pipe: false,
        }),
      ],
      copy_in,
      copy_out: vec!["stderr".to_string()],
      copy_out_cached: vec!["stdout".to_string()],
      time_limit,
      memory_limit,
      ..Default::default()
    };

    return match self.exec(vec![cmd], vec![]).await {
      // Return file id of stdout if the command executed successful.
      Ok(res) => (
        result::JudgeResult::from(res.results[0].clone()),
        match res.results[0].status() {
          proto::StatusType::Accepted => Some(res.results[0].file_ids["stdout"].clone()),
          _ => None,
        },
      ),
      // A sandbox error encountered.
      Err(e) => (
        result::JudgeResult {
          status: result::Status::SystemError,
          time: time::Duration::ZERO,
          memory: 0,
          stderr: format!("Sandbox error: {}", e),
          exit_code: -1,
        },
        None,
      ),
    };
  }
}
