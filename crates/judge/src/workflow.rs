use std::{
  collections::{HashMap, HashSet},
  fmt::Debug,
  mem,
  str::FromStr,
  sync::Arc,
  time,
};

use async_trait::async_trait;
use futures::{
  stream::{self, StreamExt},
  TryStreamExt,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationNanoSeconds};
use thiserror::Error;
use tokio::sync::watch;

use crate::{compile, etc, file, generator, judge, result, sandbox, validator};

/// A workflow to a set of tasks (like build a problem or do a stress).
#[derive(Debug, Serialize, Deserialize)]
pub struct Workflow {
  pub copy_in: HashMap<String, file::File>,

  pub tasks: Vec<Box<dyn Cmd>>,

  pub copy_out: HashSet<String>,
}

type FileSender = watch::Sender<Option<Arc<sandbox::FileHandle>>>;
type FileReceiver = watch::Receiver<Option<Arc<sandbox::FileHandle>>>;

impl Workflow {
  /// Check file relativity of targets.
  ///
  /// Returns the file senders of each task, file receivers of each task,
  /// and file receivers of global copy_out.
  fn parse(
    &self,
  ) -> Result<
    (
      HashMap<String, FileSender>,
      HashMap<String, FileReceiver>,
      Vec<HashMap<String, FileSender>>,
      Vec<HashMap<String, FileReceiver>>,
    ),
    ParseError,
  > {
    let n = self.tasks.len();
    let mut providers = HashMap::new();
    let mut file_receivers = HashMap::new();
    let mut inf_receivers = Vec::with_capacity(n);
    let mut ouf_senders = Vec::with_capacity(n);
    let mut global_inf_senders = HashMap::new();
    for _ in 0..n {
      inf_receivers.push(HashMap::new());
      ouf_senders.push(HashMap::new());
    }

    for inf in &self.copy_in {
      let (tx, rx) = watch::channel(None);
      global_inf_senders.insert(inf.0.to_string(), tx);
      file_receivers.insert(inf.0.to_string(), rx);
    }

    // Record the task index of each copy_out file,
    // and check if multiple tasks output the same file.
    for (i, cmd) in self.tasks.iter().enumerate() {
      for ouf in &cmd.get_copy_out() {
        if self.copy_in.contains_key(ouf) {
          return Err(
            DuplicateFileError::CopyIn {
              index: i,
              name: ouf.to_string(),
            }
            .into(),
          );
        }
        if let Some(prev_idx) = providers.insert(ouf.to_string(), i) {
          return Err(
            DuplicateFileError::Prev {
              index1: prev_idx,
              index2: i,
              name: ouf.to_string(),
            }
            .into(),
          );
        }
        let (tx, rx) = watch::channel(None);
        ouf_senders[i].insert(ouf.to_string(), tx);
        file_receivers.insert(ouf.to_string(), rx);
      }
    }

    // For each task, add receivers of it's input files to hash map.
    for (i, cmd) in self.tasks.iter().enumerate() {
      for inf in &cmd.get_copy_in() {
        if !self.copy_in.contains_key(inf) && !providers.contains_key(inf) {
          return Err(
            InvalidFileError::Target {
              index: i,
              name: inf.to_string(),
            }
            .into(),
          );
        }
        inf_receivers[i].insert(inf.to_string(), file_receivers[inf].clone());
      }
    }

    // Check if global copy out files a provided.
    for ouf in &self.copy_out {
      if !providers.contains_key(ouf) {
        return Err(InvalidFileError::Global(ouf.to_string()).into());
      }
    }

    return Ok((
      global_inf_senders,
      file_receivers,
      ouf_senders,
      inf_receivers,
    ));
  }

  pub async fn exec(&self) -> Result<HashMap<String, Arc<sandbox::FileHandle>>, Error> {
    let (mut global_inf_senders, mut file_receivers, mut ouf_senders, mut inf_receivers) = self
      .parse()
      .map_or_else(|e| Err(Error::Parse(e)), |g| Ok(g))?;

    // Upload files to sandbox.
    for inf in &self.copy_in {
      let content = inf.1.as_bytes();
      let file = Arc::new(sandbox::FileHandle::upload(&content).await);
      let sender = global_inf_senders.remove(inf.0).unwrap();
      _ = sender.send(Some(file));
    }

    let coroutines = futures::stream::FuturesUnordered::new();
    for (i, task) in self.tasks.iter().enumerate() {
      let ir = mem::replace(&mut inf_receivers[i], HashMap::new());
      let os = mem::replace(&mut ouf_senders[i], HashMap::new());
      let task = task.clone();
      coroutines.push(async move {
        if let Err(e) = task.exec(ir, os).await {
          return Err(Error::Execute {
            index: i,
            source: e,
          });
        }
        return Ok(());
      });
    }
    coroutines.try_collect().await?;

    let res = stream::iter(&mut file_receivers)
      .then(|f| async move {
        (f.0.to_string(), {
          f.1.changed().await.unwrap();
          let x = (*f.1.borrow()).clone();
          x.unwrap()
        })
      })
      .collect()
      .await;

    return Ok(res);
  }
}

#[derive(Debug, Error, Clone)]
pub enum Error {
  #[error("parse error")]
  Parse(#[from] ParseError),
  #[error("execute error at {index}")]
  Execute { index: usize, source: ExecuteError },
}

/// Error when parsing.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
  #[error("invalid copy in file")]
  InvalidFile(#[from] InvalidFileError),
  #[error("duplicate file")]
  DuplicateFile(#[from] DuplicateFileError),
}

/// Error when parsing.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum InvalidFileError {
  #[error("invalid copy in file at {index}: {name}")]
  Target { index: usize, name: String },
  #[error("invalid copy out file at global copy_out: {0}")]
  Global(String),
}

/// Error when parsing.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DuplicateFileError {
  #[error("duplicate global copy in and copy out file at {index}: {name}")]
  CopyIn { index: usize, name: String },
  #[error("duplicate copy out file at {index1} and {index2}: {name}")]
  Prev {
    index1: usize,
    index2: usize,
    name: String,
  },
}

/// Errors when command execute.
#[derive(Debug, Error, Clone)]
pub enum ExecuteError {
  #[error("invalid lang")]
  InvalidLang(#[from] etc::InvalidLangError),
  #[error("runtime error")]
  Runtime(#[from] result::RuntimeError),
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Cmd: std::fmt::Debug + Sync + Send {
  /// Get all copy in files of the command.
  fn get_copy_in(&self) -> HashSet<String>;

  /// Get all copy in files of the command.
  fn get_copy_out(&self) -> HashSet<String>;

  /// Execute the command.
  async fn exec(
    &self,
    copy_in_receivers: HashMap<String, FileReceiver>,
    copy_out_senders: HashMap<String, FileSender>,
  ) -> Result<(), ExecuteError>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompileCmd {
  pub lang: String,
  pub args: Vec<String>,
  pub code: String,
  /// Extra copy in file to send to the sandbox.
  pub copy_in: HashMap<String, String>,
  /// Save filename of the compiled executable file.
  pub exec: String,
}

#[async_trait]
#[typetag::serde(name = "compile")]
impl Cmd for CompileCmd {
  fn get_copy_in(&self) -> HashSet<String> {
    let mut res: HashSet<String> = self.copy_in.keys().cloned().collect();
    res.insert(self.code.clone());
    return res;
  }

  fn get_copy_out(&self) -> HashSet<String> {
    [self.exec.clone()].into()
  }

  async fn exec(
    &self,
    mut copy_in_receivers: HashMap<String, FileReceiver>,
    mut copy_out_senders: HashMap<String, FileSender>,
  ) -> Result<(), ExecuteError> {
    let lang = etc::LangCfg::from_str(&self.lang).map_or(
      Err(ExecuteError::InvalidLang(etc::InvalidLangError {
        lang: self.lang.clone(),
      })),
      |x| Ok(x),
    )?;
    let code = {
      let mut rx = copy_in_receivers.remove(&self.code).unwrap();
      rx.changed().await.unwrap();
      let x = (*rx.borrow()).clone();
      x.unwrap()
    };
    let copy_in: HashMap<_, _> = stream::iter(&self.copy_in)
      .then(|f| {
        let mut rx = copy_in_receivers.remove(f.1).unwrap();
        async move {
          (f.0.to_string(), {
            rx.changed().await.unwrap();
            let x = (*rx.borrow()).clone();
            x.unwrap()
          })
        }
      })
      .collect()
      .await;

    log::debug!("compile for {} start", &self.exec);

    let res = compile::compile(&lang, self.args.clone(), code, copy_in).await?;
    _ = copy_out_senders.remove(&self.exec).unwrap().send(Some(res));

    log::debug!("compile for {} finished", &self.exec);

    return Ok(());
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerateCmd {
  pub lang: String,
  pub args: Vec<String>,
  pub exec: String,
  /// Extra copy in file to send to the sandbox.
  pub copy_in: HashMap<String, String>,
  /// The save filename of the generated input file.
  pub generated: String,
}

#[async_trait]
#[typetag::serde(name = "generate")]
impl Cmd for GenerateCmd {
  fn get_copy_in(&self) -> HashSet<String> {
    let mut res: HashSet<String> = self.copy_in.keys().cloned().collect();
    res.insert(self.exec.clone());
    return res;
  }

  fn get_copy_out(&self) -> HashSet<String> {
    [self.generated.clone()].into()
  }

  async fn exec(
    &self,
    mut copy_in_receivers: HashMap<String, FileReceiver>,
    mut copy_out_senders: HashMap<String, FileSender>,
  ) -> Result<(), ExecuteError> {
    let lang = etc::LangCfg::from_str(&self.lang).map_or(
      Err(ExecuteError::InvalidLang(etc::InvalidLangError {
        lang: self.lang.clone(),
      })),
      |x| Ok(x),
    )?;
    let exec = {
      let mut rx = copy_in_receivers.remove(&self.exec).unwrap();
      rx.changed().await.unwrap();
      let x = (*rx.borrow()).clone();
      x.unwrap()
    };
    let copy_in: HashMap<_, _> = stream::iter(&self.copy_in)
      .then(|f| {
        let mut rx = copy_in_receivers.remove(f.1).unwrap();
        async move {
          (f.0.to_string(), {
            rx.changed().await.unwrap();
            let x = (*rx.borrow()).clone();
            x.unwrap()
          })
        }
      })
      .collect()
      .await;

    let res = generator::generate(&lang, self.args.clone(), exec, copy_in).await?;
    _ = copy_out_senders
      .remove(&self.generated)
      .unwrap()
      .send(Some(res));

    return Ok(());
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Command to run a validator.
pub struct ValidateCmd {
  /// Language of validator.
  pub lang: String,

  /// Validator run args.
  pub args: Vec<String>,

  /// Executable validator file.
  pub exec: String,

  /// Input file.
  pub inf: String,

  /// Extra copy in files.
  pub copy_in: HashMap<String, String>,

  /// Report file output name.
  pub report: String,
}

#[async_trait]
#[typetag::serde(name = "validate")]
impl Cmd for ValidateCmd {
  fn get_copy_in(&self) -> HashSet<String> {
    let mut res: HashSet<String> = self.copy_in.keys().cloned().collect();
    res.insert(self.exec.clone());
    res.insert(self.inf.clone());
    return res;
  }

  fn get_copy_out(&self) -> HashSet<String> {
    [self.report.clone()].into()
  }

  async fn exec(
    &self,
    mut copy_in_receivers: HashMap<String, FileReceiver>,
    mut copy_out_senders: HashMap<String, FileSender>,
  ) -> Result<(), ExecuteError> {
    let lang = etc::LangCfg::from_str(&self.lang).map_or(
      Err(ExecuteError::InvalidLang(etc::InvalidLangError {
        lang: self.lang.clone(),
      })),
      |x| Ok(x),
    )?;
    let exec = {
      let mut rx = copy_in_receivers.remove(&self.exec).unwrap();
      rx.changed().await.unwrap();
      let x = (*rx.borrow()).clone();
      x.unwrap()
    };
    let inf = {
      let mut rx = copy_in_receivers.remove(&self.inf).unwrap();
      rx.changed().await.unwrap();
      let x = (*rx.borrow()).clone();
      x.unwrap()
    };
    let copy_in: HashMap<_, _> = stream::iter(&self.copy_in)
      .then(|f| {
        let mut rx = copy_in_receivers.remove(f.1).unwrap();
        async move {
          (f.0.to_string(), {
            rx.changed().await.unwrap();
            let x = (*rx.borrow()).clone();
            x.unwrap()
          })
        }
      })
      .collect()
      .await;

    let overview = validator::validate(&lang, self.args.clone(), exec, inf, copy_in).await?;

    let report_file =
      Arc::new(sandbox::FileHandle::upload(&rmp_serde::to_vec(&overview).unwrap()).await);

    _ = copy_out_senders
      .remove(&self.report)
      .unwrap()
      .send(Some(report_file));

    return Ok(());
  }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JudgeBatchCmd {
  pub lang: String,
  pub args: Vec<String>,
  pub exec: String,
  pub inf: String,
  pub copy_in: HashMap<String, String>,
  pub copy_out: String,
  #[serde_as(as = "DurationNanoSeconds<u64>")]
  pub time_limit: time::Duration,
  pub memory_limit: u64,
}

#[async_trait]
#[typetag::serde(name = "judge_batch")]
impl Cmd for JudgeBatchCmd {
  fn get_copy_in(&self) -> HashSet<String> {
    let mut res: HashSet<String> = self.copy_in.keys().cloned().collect();
    res.insert(self.exec.clone());
    res.insert(self.inf.clone());
    return res;
  }

  fn get_copy_out(&self) -> HashSet<String> {
    [self.copy_out.clone()].into()
  }

  async fn exec(
    &self,
    mut copy_in_receivers: HashMap<String, FileReceiver>,
    mut copy_out_senders: HashMap<String, FileSender>,
  ) -> Result<(), ExecuteError> {
    let lang = etc::LangCfg::from_str(&self.lang).map_or(
      Err(ExecuteError::InvalidLang(etc::InvalidLangError {
        lang: self.lang.clone(),
      })),
      |x| Ok(x),
    )?;
    let exec = {
      let mut rx = copy_in_receivers.remove(&self.exec).unwrap();
      rx.changed().await.unwrap();
      let x = (*rx.borrow()).clone();
      x.unwrap()
    };
    let inf = {
      let mut rx = copy_in_receivers.remove(&self.inf).unwrap();
      rx.changed().await.unwrap();
      let x = (*rx.borrow()).clone();
      x.unwrap()
    };
    let copy_in: HashMap<_, _> = stream::iter(&self.copy_in)
      .then(|f| {
        let mut rx = copy_in_receivers.remove(f.1).unwrap();
        async move {
          (f.0.to_string(), {
            rx.changed().await.unwrap();
            let x = (*rx.borrow()).clone();
            x.unwrap()
          })
        }
      })
      .collect()
      .await;

    let (res, copy_out_file) = judge::judge_batch(
      &lang,
      self.args.clone(),
      exec,
      inf,
      copy_in,
      self.time_limit,
      self.memory_limit,
    )
    .await;

    if res.status != sandbox::Status::Accepted {
      return Err(ExecuteError::Runtime(res.into()));
    }

    _ = copy_out_senders
      .remove(&self.copy_out)
      .unwrap()
      .send(Some(copy_out_file.unwrap()));
    return Ok(());
  }
}
