use std::{collections::HashMap, sync::Arc};

use crate::{etc, result, sandbox};

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
  lang: &etc::LangCfg,
  args: Vec<String>,
  exec: Arc<sandbox::FileHandle>,
  mut copy_in: HashMap<String, Arc<sandbox::FileHandle>>,
) -> Result<Arc<sandbox::FileHandle>, result::RuntimeError> {
  copy_in.insert(lang.exec().to_string(), exec);

  let res = sandbox::Request::Run(sandbox::Cmd {
    args: [lang.run_cmd().clone(), args].concat(),
    copy_in,
    copy_out: vec!["stdout".to_string()],
    ..Default::default()
  })
  .exec()
  .await[0]
    .clone();

  match res.result.status {
    sandbox::Status::Accepted => Ok(res.files["stdout"].clone()),
    _ => Err(res.result.into()),
  }
}
