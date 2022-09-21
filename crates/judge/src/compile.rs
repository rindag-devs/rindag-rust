use std::{collections::HashMap, time};

use thiserror::Error;

use crate::{
  etc, result,
  sandbox::{self, proto},
};

/// Compile error.
#[derive(Debug, Error)]
pub enum Error {
  #[error(
    "compiler runs failed (status: {status:?}, \
    time: {time:?}, memory: {memory} bytes, message: {message})"
  )]
  Execute {
    status: proto::StatusType,
    time: time::Duration,
    memory: u64,
    message: String,
    exit_code: i32,
  },

  #[error("file error: {0:?}")]
  File(proto::FileError),

  #[error("sandbox error")]
  Sandbox(#[from] tonic::Status),
}

impl From<proto::Result> for Error {
  fn from(res: proto::Result) -> Self {
    return Self::Execute {
      status: res.status(),
      message: result::limit_message(&String::from_utf8_lossy(&res.files["stderr"])),
      memory: res.memory,
      time: time::Duration::from_nanos(res.time),
      exit_code: res.exit_status,
    };
  }
}

impl sandbox::Client {
  /// Compile the given code and return the compile result and the file id of the executable.
  ///
  /// It will do these following:
  ///
  /// 1. Constructs a sandbox request according to the code language.
  /// 2. Execute this request with sandbox.
  /// 3. Check if there's an error happens, or get the executable file id.
  ///
  /// # Errors
  ///
  /// This function will return an error if the compilation failed or
  /// a sandbox internal error was encountered.
  pub async fn compile(
    &self,
    lang: &etc::LangCfg,
    code: proto::File,
    mut copy_in: HashMap<String, proto::File>,
  ) -> Result<String, Error> {
    copy_in.insert(lang.source.clone(), code);

    let cmd = proto::Cmd {
      args: lang.compile_cmd.clone(),
      copy_in,
      copy_out: vec!["stderr".to_string()],
      copy_out_cached: vec![lang.exec.clone()],
      ..Default::default()
    };

    let res = self.exec(vec![cmd], vec![]).await;

    if let Err(e) = res {
      return Err(Error::Sandbox(e));
    }

    let res = &res.unwrap().results[0];

    if res.status() != proto::StatusType::Accepted {
      return Err(res.clone().into());
    }

    return match res.file_ids.get(&lang.exec) {
      Some(file) => Ok(file.to_string()),
      None => Err(Error::File(
        res
          .file_error
          .clone()
          .into_iter()
          .filter(|x| x.name == lang.exec)
          .last()
          .unwrap(),
      )),
    };
  }
}
