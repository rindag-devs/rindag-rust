use std::{collections::HashMap, sync::Arc};

use crate::{generator, result, sandbox};

/// Input of test case.
#[derive(Debug, Clone)]
pub enum Input {
  /// Generated input.
  Generated {
    generator: generator::Generator,
    args: Vec<String>,
  },

  /// Plain text input file.
  Plain { context: Vec<u8> },
}

impl Input {
  /// Make the input and upload to sandbox.
  pub async fn make(
    &self,
    copy_in: HashMap<String, Arc<sandbox::FileHandle>>,
  ) -> Result<Arc<sandbox::FileHandle>, result::RuntimeError> {
    match self {
      Input::Generated { generator, args } => generator.generate(args.clone(), copy_in).await,
      Input::Plain { context } => Ok(Arc::new(sandbox::FileHandle::upload(context).await)),
    }
  }
}
