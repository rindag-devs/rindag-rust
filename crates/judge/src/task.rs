use std::collections::HashMap;

use crate::{
  etc, result,
  sandbox::{client, exec},
};

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

  let mut client = client::CLIENT.get().await.borrow_mut();
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
    result::CompileResult {
      status: res.results[0].status,
      stderr: res.results[0].files["stderr"].clone(),
      stdout: res.results[0].files["stdout"].clone(),
    },
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
