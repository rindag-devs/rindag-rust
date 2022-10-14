use std::{collections::HashMap, sync::Arc};

use crate::{
  etc, result,
  sandbox::{self, proto},
};

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
    args: Vec<String>,
    code: proto::File,
    mut copy_in: HashMap<String, proto::File>,
  ) -> Result<String, result::Error> {
    copy_in.insert(lang.source.clone(), code);

    let cmd = proto::Cmd {
      args: [lang.compile_cmd.clone(), args].concat(),
      copy_in,
      copy_out: vec!["stderr".to_string()],
      copy_out_cached: vec![lang.exec.clone()],
      ..Default::default()
    };

    let res = self.exec(vec![cmd], vec![]).await;

    if let Err(e) = res {
      return Err(result::Error::Sandbox(Arc::new(e)));
    }

    let res = &res.unwrap().results[0];

    if res.status() != proto::StatusType::Accepted {
      return Err(res.clone().into());
    }

    return Ok(res.file_ids[&lang.exec].clone());
  }
}
