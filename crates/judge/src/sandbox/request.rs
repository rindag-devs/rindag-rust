use core::time;
use std::{collections::HashMap, sync::Arc};

use crate::CONFIG;

use super::{client, file::FileHandle, proto, ResponseResult};

/// A sandbox judge request is a request to run some commands in sandbox.
#[derive(Debug, Clone)]
pub enum Request {
  /// Run a single command.
  Run(Cmd),

  /// Run two commands, which use pipe to connect input and output streams to each other.
  RunPiped([Cmd; 2]),
}

impl Request {
  /// Convert a wrapped request to sandbox proto request.
  fn to_proto_request(&self) -> proto::Request {
    let c = &CONFIG.judge;
    match self {
      Request::Run(cmd) => proto::Request {
        cmd: vec![proto::request::CmdType {
          args: cmd.args.clone(),
          env: [c.env.clone(), cmd.env.clone()].concat(),
          files: vec![
            match &cmd.stdin {
              Some(f) => proto::request::File {
                file: Some(proto::request::file::File::Cached(
                  proto::request::CachedFile {
                    file_id: f.id.clone(),
                  },
                )),
              },
              None => proto::request::File {
                file: Some(proto::request::file::File::Memory(
                  proto::request::MemoryFile {
                    content: "".as_bytes().to_vec(),
                  },
                )),
              },
            },
            proto::request::File {
              file: Some(proto::request::file::File::Pipe(
                proto::request::PipeCollector {
                  name: "stdout".to_string(),
                  max: c.stdout_limit,
                  pipe: false,
                },
              )),
            },
            proto::request::File {
              file: Some(proto::request::file::File::Pipe(
                proto::request::PipeCollector {
                  name: "stderr".to_string(),
                  max: c.stderr_limit,
                  pipe: false,
                },
              )),
            },
          ],
          tty: false,
          cpu_time_limit: cmd.time_limit.as_nanos().try_into().unwrap(),
          clock_time_limit: (cmd.time_limit.as_nanos() as f64 * 2.).ceil() as u64,
          memory_limit: cmd.memory_limit,
          stack_limit: cmd.memory_limit,
          proc_limit: c.process_limit,
          strict_memory_limit: false,
          copy_in: cmd
            .copy_in
            .iter()
            .map(|f| {
              {
                (
                  f.0.clone(),
                  proto::request::File {
                    file: Some(proto::request::file::File::Cached(
                      proto::request::CachedFile {
                        file_id: f.1.id.clone(),
                      },
                    )),
                  },
                )
              }
            })
            .collect(),
          copy_out: vec![],
          copy_out_cached: cmd
            .copy_out
            .iter()
            .map(|f| proto::request::CmdCopyOutFile {
              name: f.to_string(),
              optional: false,
            })
            .collect(),
          ..Default::default()
        }],
        pipe_mapping: vec![],
        ..Default::default()
      },
      // TODO: be used in interactive problems.
      Request::RunPiped(_) => todo!(),
    }
  }

  pub async fn exec(&self) -> Vec<ResponseResult> {
    let resp = client::CLIENT
      .get()
      .await
      .exec(self.to_proto_request())
      .await;
    if !resp.error.is_empty() {
      panic!("sandbox execute returns an error: {}", resp.error);
    }
    return resp.results.into_iter().map(ResponseResult::from).collect();
  }
}

/// A command to judge in sandbox.
#[derive(Debug, Clone)]
pub struct Cmd {
  /// Command line argument.
  pub args: Vec<String>,

  /// Environment variables.
  pub env: Vec<String>,

  /// Time limit to run this command.
  pub time_limit: time::Duration,

  /// Memory limit in byte.
  pub memory_limit: u64,

  /// Stdin of the file.
  ///
  /// If this command is used in a piped execution, leave this field to None.
  ///
  /// If this field is None the command is used in a `Request::Run`, it will use a empty file.
  pub stdin: Option<Arc<FileHandle>>,

  /// Copy the correspond file to the container dst path.
  pub copy_in: HashMap<String, Arc<FileHandle>>,

  /// Names to files which is need to be copied out from the container after execution.
  ///
  /// Append '?' after file name will make the file optional and do not cause FileError when missing.
  pub copy_out: Vec<String>,
}

impl Default for Cmd {
  fn default() -> Self {
    let c = &CONFIG.judge;
    Self {
      args: vec![],
      env: vec![],
      time_limit: c.time_limit,
      memory_limit: c.memory_limit,
      stdin: None,
      copy_in: [].into(),
      copy_out: vec![],
    }
  }
}
