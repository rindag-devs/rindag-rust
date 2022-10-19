use std::{collections::HashMap, sync::Arc};

use crate::{
  etc, result,
  sandbox::{self, proto},
};

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
  pub async fn generate(
    &self,
    lang: &etc::LangCfg,
    args: Vec<String>,
    exec: proto::File,
    mut copy_in: HashMap<String, proto::File>,
  ) -> Result<String, result::Error> {
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
        _ => {
          if let Some(stdout_id) = res.results[0].file_ids.get("stdout") {
            _ = self.file_delete(stdout_id.to_string()).await;
          }
          Err(res.results[0].clone().into())
        }
      },
      Err(e) => Err(result::Error::Sandbox(Arc::new(e))),
    };
  }
}
