use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{data, lang, result, sandbox};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Source {
  pub lang: lang::Lang,
  pub data: data::Provider,
}

#[derive(Debug, Clone)]
pub struct Executable {
  pub lang: lang::Lang,
  pub file: Arc<sandbox::FileHandle>,
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
    mut copy_in: HashMap<String, Arc<sandbox::FileHandle>>,
  ) -> Result<Executable, result::RuntimeError> {
    copy_in.insert(
      self.lang.source().to_string(),
      Arc::new(sandbox::FileHandle::upload(&self.data.as_bytes()).await),
    );

    let mut res = sandbox::Request::Run(sandbox::Cmd {
      args: [self.lang.compile_cmd().clone(), args].concat(),
      copy_in,
      copy_out: vec![self.lang.exec().to_string()],
      ..Default::default()
    })
    .exec()
    .await;

    assert_eq!(res.len(), 1);
    let res = res.pop().unwrap();

    if res.result.status != sandbox::Status::Accepted {
      return Err(res.result.into());
    }

    Ok(Executable {
      lang: self.lang.clone(),
      file: res.files[self.lang.exec()].clone(),
    })
  }
}
