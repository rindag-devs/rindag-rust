tonic::include_proto!("pb");

use std::{collections::HashMap, time};

use crate::CONFIG;

pub use self::{
  executor_client::ExecutorClient,
  request::{
    file::File, CachedFile, CmdCopyOutFile, CmdType, LocalFile, MemoryFile, PipeCollector, PipeMap,
    StreamInput, StreamOutput,
  },
  response::{file_error::ErrorType, result::StatusType, FileError, Result},
};

#[derive(Debug, Clone)]
pub struct Cmd {
  /// command line argument
  pub args: Vec<String>,

  /// environment
  pub env: Vec<String>,

  /// specifies file input / pipe collector for program file descriptors
  pub files: Vec<File>,

  /// enables tty on the input and output pipes (should have just one input & one output)
  ///
  /// Notice: must have TERM environment variables (e.g. TERM=xterm)
  pub tty: bool,

  /// CPU time limit.
  ///
  /// Real time limit = CPU time limit * 2.
  pub time_limit: time::Duration,

  /// byte
  pub memory_limit: u64,

  /// process count limit
  pub proc_limit: u64,

  /// Linux only: use stricter memory limit (+ rlimit_data when cgroup enabled)
  pub strict_memory_limit: bool,

  /// copy the correspond file to the container dst path
  pub copy_in: HashMap<String, File>,

  /// copy out specifies files need to be copied out from the container after execution
  ///
  /// append '?' after file name will make the file optional and do not cause FileError when missing
  pub copy_out: Vec<String>,

  /// similar to copyOut but stores file in executor service and returns file id,
  ///
  /// later download through /file/:fileId
  pub copy_out_cached: Vec<String>,
}

impl Default for Cmd {
  fn default() -> Self {
    let c = &CONFIG.sandbox;
    Cmd {
      args: vec![],
      env: c.env.clone(),
      files: vec![
        File::Memory(MemoryFile { content: vec![] }),
        File::Pipe(PipeCollector {
          name: "stdout".to_string(),
          max: c.stdout_limit,
          pipe: false,
        }),
        File::Pipe(PipeCollector {
          name: "stderr".to_string(),
          max: c.stderr_limit,
          pipe: false,
        }),
      ],
      tty: false,
      time_limit: c.time_limit,
      memory_limit: c.memory_limit,
      proc_limit: c.process_limit,
      strict_memory_limit: false,
      copy_in: HashMap::new(),
      copy_out: vec!["stderr".to_string()],
      copy_out_cached: vec![],
    }
  }
}

impl From<Cmd> for CmdType {
  fn from(cmd: Cmd) -> Self {
    CmdType {
      args: cmd.args,
      env: cmd.env,
      files: cmd
        .files
        .into_iter()
        .map(|f| request::File { file: Some(f) })
        .collect(),
      tty: cmd.tty,
      cpu_time_limit: cmd.time_limit.as_nanos() as u64,
      clock_time_limit: cmd.time_limit.as_nanos() as u64 * 2,
      memory_limit: cmd.memory_limit,
      stack_limit: cmd.memory_limit,
      proc_limit: cmd.proc_limit,
      strict_memory_limit: cmd.strict_memory_limit,
      copy_in: cmd
        .copy_in
        .into_iter()
        .map(|f| (f.0, request::File { file: Some(f.1) }))
        .collect(),
      copy_out: cmd
        .copy_out
        .into_iter()
        .map(|mut name| {
          let optional = name.ends_with("?");
          optional.then(|| name.pop());
          CmdCopyOutFile { name, optional }
        })
        .collect(),
      copy_out_cached: cmd
        .copy_out_cached
        .into_iter()
        .map(|mut name| {
          let optional = name.ends_with("?");
          optional.then(|| name.pop());
          CmdCopyOutFile { name, optional }
        })
        .collect(),
      ..Default::default()
    }
  }
}
