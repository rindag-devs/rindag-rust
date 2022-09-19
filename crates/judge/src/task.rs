use std::{collections::HashMap, time};

use crate::{etc, result, sandbox::proto, CLIENT, CONFIG};

/// Compile the given code and returns the compile result.
pub async fn compile(
  lang: &etc::LangCfg,
  code: proto::File,
  mut copy_in: HashMap<String, proto::File>,
) -> (result::CompileResult, Result<String, proto::FileError>) {
  copy_in.insert(lang.source.clone(), code);

  let cmd = proto::Cmd {
    args: lang.compile_cmd.clone(),
    copy_in,
    copy_out: vec!["stdout".to_string(), "stderr".to_string()],
    copy_out_cached: vec![lang.exec.clone()],
    ..Default::default()
  };

  let client = CLIENT.get().await.as_ref();
  let rx = client.exec(vec![cmd], vec![]).await;

  let res = rx.await.unwrap().unwrap();

  if res.results.len() != 1 {
    let err_msg = format!("Sandbox error: {}", res.error);
    return (
      result::CompileResult {
        status: proto::StatusType::InternalError,
        stderr: err_msg.clone(),
        stdout: "".to_string(),
      },
      Err(proto::FileError {
        r#type: proto::ErrorType::CopyOutOpen as i32,
        name: lang.exec.clone(),
        message: err_msg,
      }),
    );
  }

  return (
    result::CompileResult::from(&res.results[0]),
    match res.results[0].file_ids.get(&lang.exec) {
      Some(file) => Ok(file.to_string()),
      None => Err(
        res.results[0]
          .file_error
          .clone()
          .into_iter()
          .filter(|x| x.name == lang.exec)
          .last()
          .unwrap(),
      ),
    },
  );
}

/// Run the given executable file on a test case of batch problem (aka. traditional problem),
/// and then returns the judgement result and the output file.
pub async fn judge_batch(
  lang: &etc::LangCfg,
  proto: proto::File,
  inf: proto::File,
  mut copy_in: HashMap<String, proto::File>,
) -> (result::JudgeResult, Result<String, proto::FileError>) {
  let c = &CONFIG.sandbox;

  copy_in.insert(lang.exec.clone(), proto);

  let cmd = proto::Cmd {
    args: lang.run_cmd.clone(),
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
    ..Default::default()
  };

  let client = CLIENT.get().await.as_ref();
  let rx = client.exec(vec![cmd], vec![]).await;

  let res = rx.await.unwrap().unwrap();

  if res.results.len() != 1 {
    let err_msg = format!("Sandbox error: {}", res.error);
    return (
      result::JudgeResult {
        status: result::Status::SystemError,
        time: time::Duration::ZERO,
        memory: 0,
        stderr: err_msg.clone(),
      },
      Err(proto::FileError {
        r#type: proto::ErrorType::CopyOutOpen as i32,
        name: "stdout".to_string(),
        message: err_msg,
      }),
    );
  }

  return (
    result::JudgeResult::from(&res.results[0]),
    match res.results[0].file_ids.get("stdout") {
      Some(file) => Ok(file.to_string()),
      None => Err(
        res.results[0]
          .file_error
          .clone()
          .into_iter()
          .filter(|x| x.name == "stdout")
          .last()
          .unwrap(),
      ),
    },
  );
}
