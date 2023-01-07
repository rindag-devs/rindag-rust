use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{data, error, lang, sandbox};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Source {
  pub lang: lang::Lang,
  pub data: data::Provider,
}

#[derive(Debug, Clone)]
pub struct Executable {
  pub lang: lang::Lang,
  pub file: sandbox::FileHandle,
}

impl Source {
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
    args: Vec<String>,
    mut copy_in: HashMap<String, sandbox::FileHandle>,
  ) -> Result<Executable, error::CompileError> {
    copy_in.insert(
      self.lang.source().to_string(),
      sandbox::FileHandle::upload(&self.data.as_bytes()).await,
    );

    let mut res = sandbox::Request::Run(sandbox::Cmd {
      args: [self.lang.compile_cmd().clone(), args].concat(),
      copy_in,
      copy_out: vec!["stderr".to_string(), self.lang.exec().to_string()],
      ..Default::default()
    })
    .exec()
    .await;

    assert_eq!(res.len(), 1);
    let res = res.pop().unwrap();

    if res.result.status != sandbox::Status::Accepted {
      return Err(error::CompileError {
        result: res.result,
        message: match res.files.get("stderr") {
          Some(message_file) => message_file
            .context()
            .await
            .map_or("broken message".to_string(), |chars| {
              String::from_utf8_lossy(&chars).to_string()
            }),
          None => "no compile message".to_string(),
        },
      });
    }

    Ok(Executable {
      lang: self.lang.clone(),
      file: res.files[self.lang.exec()].clone(),
    })
  }
}
