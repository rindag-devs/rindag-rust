use std::{collections::HashMap, time};

use thiserror::Error;

use crate::{
  etc, result,
  sandbox::{self, proto},
};

/// Error when the generator behaves abnormally.
///
/// Such as being compile limit exceed or signaled.
#[derive(Debug, Error)]
pub enum Error {
  #[error(
    "generator runs failed (status: {status:?}, \
    time: {time:?}, memory: {memory} bytes, message: {message})"
  )]
  Execute {
    status: proto::StatusType,
    time: time::Duration,
    memory: u64,
    message: String,
    exit_code: i32,
  },

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
  /// Run the generator and returns the file id of generated output.
  ///
  /// It will do these following:
  ///
  /// 1. Constructs a sandbox request according to the generator language.
  /// 2. Execute this request with sandbox.
  /// 3. Check if there's an error happens, or get the file id of generated output.
  ///
  /// # Errors
  ///
  /// This function will return an error if the generating failed or
  /// a sandbox internal error was encountered.
  pub async fn run_generator(
    &self,
    lang: &etc::LangCfg,
    args: Vec<String>,
    exec: proto::File,
    mut copy_in: HashMap<String, proto::File>,
  ) -> Result<String, Error> {
    copy_in.insert(lang.exec.clone(), exec);

    let cmd = proto::Cmd {
      args: [lang.run_cmd.clone(), args].concat(),
      copy_in,
      copy_out_cached: vec!["stdout".to_string()],
      ..Default::default()
    };

    return match self.exec(vec![cmd], vec![]).await {
      Ok(res) => match res.results[0].status() {
        proto::StatusType::Accepted => Ok(res.results[0].file_ids["stdout"].clone()),
        _ => Err(res.results[0].clone().into()),
      },
      Err(e) => Err(Error::Sandbox(e)),
    };
  }
}
