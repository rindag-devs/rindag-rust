use std::{collections::HashMap, sync::Arc};

use crate::{etc, result, sandbox};

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
  lang: &etc::LangCfg,
  args: Vec<String>,
  code: Arc<sandbox::FileHandle>,
  mut copy_in: HashMap<String, Arc<sandbox::FileHandle>>,
) -> Result<Arc<sandbox::FileHandle>, result::RuntimeError> {
  copy_in.insert(lang.source.clone(), code);

  let res = sandbox::Request::Run(sandbox::Cmd {
    args: [lang.compile_cmd.clone(), args].concat(),
    copy_in,
    copy_out: vec![lang.exec.clone()],
    ..Default::default()
  })
  .exec()
  .await[0]
    .clone();

  if res.result.status != sandbox::Status::Accepted {
    return Err(res.result.into());
  }

  return Ok(res.files[&lang.exec].clone());
}
