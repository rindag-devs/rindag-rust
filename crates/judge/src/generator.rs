use std::{collections::HashMap, sync::Arc};

use crate::{program, result, sandbox};

#[derive(Debug, Clone)]
pub struct Generator {
  pub exec: program::Executable,
}

impl From<program::Executable> for Generator {
  fn from(exec: program::Executable) -> Self {
    Self { exec }
  }
}

impl Generator {
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
  pub async fn generate(
    &self,
    args: Vec<String>,
    mut copy_in: HashMap<String, Arc<sandbox::FileHandle>>,
  ) -> Result<Arc<sandbox::FileHandle>, result::RuntimeError> {
    copy_in.insert(self.exec.lang.exec().to_string(), self.exec.file.clone());

    let mut res = sandbox::Request::Run(sandbox::Cmd {
      args: [self.exec.lang.run_cmd().clone(), args].concat(),
      copy_in,
      copy_out: vec!["stdout".to_string()],
      ..Default::default()
    })
    .exec()
    .await;

    assert_eq!(res.len(), 1);
    let res = res.pop().unwrap();

    match res.result.status {
      sandbox::Status::Accepted => Ok(res.files["stdout"].clone()),
      _ => Err(res.result.into()),
    }
  }
}
