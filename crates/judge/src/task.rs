use std::{collections::HashMap, time};

use crate::{etc, result, sandbox::exec, CLIENT, CONFIG};

/// Compile the given code and returns the compile result.
pub async fn compile(
  lang: &etc::LangCfg,
  code: exec::File,
  mut copy_in: HashMap<String, exec::File>,
) -> (result::CompileResult, Result<String, exec::FileError>) {
  copy_in.insert(lang.source.clone(), code);

  let cmd = exec::Cmd {
    args: lang.compile_cmd.clone(),
    copy_in,
    copy_out: vec!["stdout".to_string(), "stderr".to_string()],
    copy_out_cached: vec![lang.exec.clone()],
    ..Default::default()
  };

  let mut client = CLIENT.get().await.borrow_mut();
  let (_, rx) = client.run(vec![cmd], vec![]).await;

  let res = rx.await.unwrap();

  if res.results.len() != 1 {
    let err_msg = format!(
      "Sandbox error: {}",
      res.error.unwrap_or("No error message".to_string())
    );
    return (
      result::CompileResult {
        status: exec::Status::InternalError,
        stderr: err_msg.clone(),
        stdout: "".to_string(),
      },
      Err(exec::FileError {
        error_type: exec::FileErrorType::CopyOutOpen,
        name: lang.exec.clone(),
        message: Some(err_msg),
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
  exec: exec::File,
  inf: exec::File,
  mut copy_in: HashMap<String, exec::File>,
) -> (result::JudgeResult, Result<String, exec::FileError>) {
  let c = &CONFIG.read().unwrap().sandbox;

  copy_in.insert(lang.exec.clone(), exec);

  let cmd = exec::Cmd {
    args: lang.run_cmd.clone(),
    files: vec![
      inf,
      exec::File::Collector {
        name: "stdout".to_string(),
        max: c.stdout_limit,
        pipe: false,
      },
      exec::File::Collector {
        name: "stderr".to_string(),
        max: c.stderr_limit,
        pipe: false,
      },
    ],
    copy_in,
    copy_out: vec!["stderr".to_string()],
    copy_out_cached: vec!["stdout".to_string()],
    ..Default::default()
  };

  let mut client = CLIENT.get().await.borrow_mut();
  let (_, rx) = client.run(vec![cmd], vec![]).await;

  let res = rx.await.unwrap();

  if res.results.len() != 1 {
    let err_msg = format!(
      "Sandbox error: {}",
      res.error.unwrap_or("No error message".to_string())
    );
    return (
      result::JudgeResult {
        status: result::Status::SystemError,
        time: time::Duration::ZERO,
        memory: 0,
        stderr: err_msg.clone(),
      },
      Err(exec::FileError {
        error_type: exec::FileErrorType::CopyOutOpen,
        name: "stdout".to_string(),
        message: Some(err_msg),
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
