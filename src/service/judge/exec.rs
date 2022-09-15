use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::service::etc::CONFIG;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum File {
  Local {
    /// Absolute path for the file.
    src: String,
  },

  Memory {
    /// File contents.
    ///
    /// Due to the implementation of go-judge, content can only be `String`,
    /// If binary files are required, use prepared files.
    content: String,
  },

  #[serde(rename_all = "camelCase")]
  Prepared {
    /// file_id defines file uploaded by `/file`.
    file_id: String,
  },

  Collector {
    /// file name in `copy_out`
    name: String,
    /// maximum bytes to collect from pipe
    max: u64,
    /// collect over pipe or not (default false)
    pipe: bool,
  },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
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

  /// ns
  pub cpu_limit: u64,

  /// ns
  pub clock_limit: u64,

  /// byte
  pub memory_limit: u64,

  /// byte (N/A on windows, macOS cannot set over 32M)
  pub stack_limit: u64,
  pub proc_limit: u64,

  /// limit cpu usage (1000 equals 1 cpu)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cpu_rate_limit: Option<u64>,

  /// Linux only: set the cpuSet for cgroup
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cpu_set_limit: Option<String>,

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

  /// specifies the directory to dump container /w content
  #[serde(skip_serializing_if = "Option::is_none")]
  pub copy_out_dir: Option<String>,

  /// specifies the max file size to copy out
  ///
  /// byte
  #[serde(skip_serializing_if = "Option::is_none")]
  pub copy_out_max: Option<u64>,
}

impl Default for Cmd {
  fn default() -> Self {
    let c = &CONFIG.read().unwrap().judge;
    Cmd {
      args: vec![],
      env: c.env.clone(),
      files: vec![
        File::Memory {
          content: "".to_string(),
        },
        File::Collector {
          name: "stdout".to_string(),
          max: c.stdout_limit,
          pipe: false,
        },
        File::Collector {
          name: "stderr".to_string(),
          max: c.stderr_limit,
          pipe: false,
        },
      ],
      tty: false,
      cpu_limit: c.time_limit.as_nanos() as u64,
      clock_limit: c.time_limit.as_nanos() as u64 * 2,
      memory_limit: c.memory_limit,
      stack_limit: c.memory_limit,
      proc_limit: c.process_limit,
      cpu_rate_limit: None,
      cpu_set_limit: None,
      strict_memory_limit: false,
      copy_in: HashMap::new(),
      copy_out: vec!["stderr".to_string()],
      copy_out_cached: vec![],
      copy_out_dir: None,
      copy_out_max: None,
    }
  }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Copy)]
pub enum Status {
  Accepted,
  #[serde(alias = "Memory Limit Exceeded")]
  MemoryLimitExceeded, // mle
  #[serde(alias = "Time Limit Exceeded")]
  TimeLimitExceeded, // tle
  #[serde(alias = "Output Limit Exceeded")]
  OutputLimitExceeded, // ole
  #[serde(alias = "File Error")]
  FileError, // fe
  #[serde(alias = "Nonzero Exit Status")]
  NonzeroExitStatus,
  Signalled,
  #[serde(alias = "Internal Error")]
  InternalError, // system error
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum FileErrorType {
  CopyInOpenFile,
  CopyInCreateFile,
  CopyInCopyContent,
  CopyOutOpen,
  CopyOutNotRegularFile,
  CopyOutSizeExceeded,
  CopyOutCreateFile,
  CopyOutCopyContent,
  CollectSizeExceeded,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct PipeIndex {
  pub index: u64, // the index of cmd
  pub fd: u64,    // the fd number of cmd
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PipeMap {
  #[serde(alias = "in")]
  /// input end of the pipe
  pub inp: PipeIndex,

  /// output end of the pipe
  pub out: PipeIndex,

  /// enable pipe proxy from in to out,
  ///
  /// content from in will be discarded if out closes
  pub proxy: bool,

  /// copy out proxy content if proxy enabled
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,

  /// limit the copy out content size,
  ///
  /// proxy will still functioning after max
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileError {
  /// error file name
  pub name: String,

  /// type
  #[serde(alias = "type")]
  pub error_type: FileErrorType,

  /// detailed message
  pub message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Request {
  pub request_id: uuid::Uuid, // for WebSocket requests
  pub cmd: Vec<Cmd>,
  pub pipe_mapping: Vec<PipeMap>,
}

impl Request {
  pub fn new(cmd: Vec<Cmd>, pipe_mapping: Vec<PipeMap>) -> Self {
    return Request {
      request_id: uuid::Uuid::new_v4(),
      cmd,
      pipe_mapping,
    };
  }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct CancelRequest {
  pub cancel_request_id: uuid::Uuid,
}

// WebSocket request
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum WSRequest {
  Request(Request),
  CancelRequest(CancelRequest),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Result {
  pub status: Status,

  /// potential system error message
  pub error: Option<String>,

  pub exit_status: u32,

  /// ns (cgroup recorded time)
  pub time: u64,

  /// byte
  pub memory: u64,

  /// ns (wall clock time)
  pub run_time: u64,

  /// copyFile name -> content
  #[serde(default)]
  pub files: HashMap<String, String>,

  /// copyFileCached name -> fileId
  #[serde(default)]
  pub file_ids: HashMap<String, String>,

  /// file_error contains detailed file errors
  #[serde(default)]
  pub file_error: Vec<FileError>,
}

/// WebSocket results.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WSResult {
  pub request_id: uuid::Uuid,
  pub results: Vec<Result>,
  pub error: Option<String>,
}
