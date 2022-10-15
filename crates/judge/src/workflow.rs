use std::{
  collections::{HashMap, HashSet},
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex, RwLock,
  },
  time,
};

use async_trait::async_trait;
use dyn_clone::DynClone;
use futures::{
  stream::{self, StreamExt},
  Future,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationNanoSeconds};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::{
  builtin, result,
  sandbox::{self, proto},
  CONFIG,
};

/// A workflow to a set of tasks (like build a problem or do a stress).
#[derive(Debug, Serialize, Deserialize)]
pub struct Workflow {
  pub copy_in: HashMap<String, File>,

  pub tasks: Vec<Box<dyn Cmd>>,

  pub copy_out: Vec<String>,
}

/// Topo sort of directed graph.
///
/// Returns `(finished, order)`.
///
/// - `finished` means if the topo sort has not been aborted.
/// - `order` is the topo order of a graph. -1 means this node is in a circle.
///
/// Call the callback function when a new node pop from queue:
/// `cb(node_index, topo_order) -> is_continue`.
///
/// If callback func return value is false, it will abort the topo sort.
async fn topo_sort<Fut: Future<Output = bool> + Send>(
  edge: Arc<Vec<Vec<usize>>>,
  cb: impl FnOnce(usize, isize) -> Fut + Send + Sync + Clone + 'static,
) -> (bool, Vec<isize>) {
  let n = edge.len();
  let mut order = vec![-1isize; n];
  let mut deg = vec![0usize; n];

  for f in 0..n {
    for t in &edge[f] {
      deg[*t] += 1;
    }
  }

  let mut tim = 0;
  let unsolved = Arc::new(AtomicUsize::new(0)); // Unsolved node count.
  let (tx, mut rx) = mpsc::unbounded_channel(); // The channel here acts like a queue.

  enum Sign {
    Next(usize), // Work with a new node.
    Finished,    // Finish topo sort.
    Aborted,     // Abort topo sort.
  }

  {
    let tx = tx.clone();
    let unsolved = unsolved.clone();
    for i in 0..n {
      if deg[i] == 0 {
        unsolved.fetch_add(1, Ordering::SeqCst);
        _ = tx.send(Sign::Next(i));
      }
    }
  }

  let deg = Arc::new(Mutex::new(deg));

  loop {
    match rx.recv().await.unwrap() {
      Sign::Next(front) => {
        let edge = edge.clone();
        let tx = tx.clone();
        let cb = cb.clone();
        let deg = deg.clone();
        let unsolved = unsolved.clone();

        order[front] = tim;
        tim += 1;
        let tim = tim.clone();

        tokio::spawn(async move {
          if !cb(front, tim).await {
            _ = tx.send(Sign::Aborted);
            return;
          }
          {
            let mut deg = deg.lock().unwrap();
            for to in &edge[front] {
              deg[*to] -= 1;
              log::debug!("{} -> {} del {}", front, to, deg[*to]);
              if deg[*to] == 0 {
                unsolved.fetch_add(1, Ordering::SeqCst);
                _ = tx.send(Sign::Next(*to));
              }
            }
          }
          if unsolved.fetch_sub(1, Ordering::SeqCst) == 1 {
            _ = tx.send(Sign::Finished);
            return;
          }
        });
      }
      Sign::Finished => return (true, order),
      Sign::Aborted => return (false, order),
    };
  }
}

impl Workflow {
  /// Check file relativity of targets.
  ///
  /// Returns a DAG of the tasks or a parsing error.
  async fn parse(&self) -> Result<Vec<Vec<usize>>, ParseError> {
    let n = self.tasks.len();
    let mut fa = HashMap::<String, usize>::new();
    let mut edge: Vec<Vec<usize>> = vec![Vec::new(); n];

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
        if let Some(prev_idx) = fa.insert(ouf.to_string(), i) {
          return Err(
            DuplicateFileError::Prev {
              index1: prev_idx,
              index2: i,
              name: ouf.to_string(),
            }
            .into(),
          );
        }
      }
    }

    // Add edge from task of copy_out to task of copy_in.
    for (i, cmd) in self.tasks.iter().enumerate() {
      for inf in &cmd.get_copy_in() {
        if self.copy_in.contains_key(inf) {
          continue;
        }
        let mat = fa.get(inf).map_or(
          Err(ParseError::InvalidFile {
            index: i,
            name: inf.to_string(),
          }),
          |f| Ok(f),
        )?;
        edge[*mat].push(i);
      }
    }
    let edge = Arc::new(edge);
    let order = topo_sort(edge.clone(), |_, _| async { true }).await.1;
    let edge = edge.to_vec();

    for i in 0..n {
      // It's impossible to become a circle.
      assert_ne!(order[i], -1);
    }

    return Ok(edge);
  }
}

impl sandbox::Client {
  pub async fn exec_workflow(
    self: &Arc<Self>,
    wf: Arc<Workflow>,
  ) -> Result<HashMap<String, String>, Error> {
    // Upload files to sandbox.
    let this = self.clone();
    let file_map = Arc::new(RwLock::new(
      stream::iter(wf.clone().copy_in.clone())
        .then(|f| async {
          (
            f.0,
            this
              .file_add(match f.1 {
                File::Memory(m) => m.to_vec(),
                File::Builtin(b) => b.content.to_vec(),
              })
              .await,
          )
        })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(|f| match f.1 {
          Ok(x) => Ok((f.0, x)),
          Err(e) => Err(Error::Sandbox(Arc::new(e))),
        })
        .collect::<Result<HashMap<_, _>, _>>()?,
    ));

    let edge = Arc::new(
      wf.parse()
        .await
        .map_or_else(|e| Err(Error::Parse(e)), |g| Ok(g))?,
    );

    let err = Arc::new(Mutex::new(None));
    let this = self.clone();
    {
      let err = err.clone();
      let file_map = file_map.clone();
      topo_sort(edge, |idx, ord| async move {
        log::info!("running task {} order {}", idx, ord);
        if let Err(e) = wf.tasks[idx].exec(this.as_ref(), file_map.clone()).await {
          *err.lock().unwrap() = Some(Error::Execute {
            index: idx,
            source: e,
          });
          return false;
        }
        return true;
      })
      .await;
    }

    return match &*err.lock().unwrap() {
      Some(err) => Err(err.clone()),
      None => Ok((&*file_map.read().unwrap()).clone()),
    };
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum File {
  #[serde(with = "serde_bytes")]
  Memory(Vec<u8>),
  Builtin(builtin::File),
}

#[derive(Debug, Error, Clone)]
pub enum Error {
  #[error("parse error")]
  Parse(#[from] ParseError),
  #[error("execute error at {index}")]
  Execute { index: usize, source: ExecuteError },
  #[error("sandbox error")]
  Sandbox(Arc<tonic::Status>),
}

/// Error when parsing.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
  #[error("invalid copy in file at {index}: {name}")]
  InvalidFile { index: usize, name: String },
  #[error("duplicate file")]
  DuplicateFile(#[from] DuplicateFileError),
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
  #[error("invalid lang: {0}")]
  InvalidLang(String),
  #[error("runtime error")]
  Runtime(#[from] result::Error),
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Cmd: std::fmt::Debug + Sync + Send + DynClone {
  /// Get all copy in files of the command.
  fn get_copy_in(&self) -> HashSet<String>;

  /// Get all copy in files of the command.
  fn get_copy_out(&self) -> HashSet<String>;

  /// Execute the command.
  async fn exec(
    &self,
    sandbox: &sandbox::Client,
    file_map: Arc<RwLock<HashMap<String, String>>>,
  ) -> Result<(), ExecuteError>;
}
dyn_clone::clone_trait_object!(Cmd);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompileCmd {
  pub lang: String,
  pub args: Vec<String>,
  pub code: String,
  pub copy_in: HashMap<String, String>,
  pub copy_out: String,
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
    [self.copy_out.clone()].into()
  }

  async fn exec(
    &self,
    sandbox: &sandbox::Client,
    file_map: Arc<RwLock<HashMap<String, String>>>,
  ) -> Result<(), ExecuteError> {
    let lang = CONFIG
      .lang
      .get(&self.lang)
      .map_or(Err(ExecuteError::InvalidLang(self.lang.clone())), |x| Ok(x))?;
    let code;
    let copy_in: HashMap<String, proto::File>;
    {
      let file_map = file_map.read().unwrap();
      code = proto::File::Cached(file_map[&self.code].clone().into());
      copy_in = self
        .copy_in
        .iter()
        .map(|f| {
          (
            f.0.to_string(),
            proto::File::Cached(file_map[f.1].clone().into()),
          )
        })
        .collect();
    }

    log::info!("compile for {} start", &self.copy_out);
    let res = sandbox
      .compile(lang, self.args.clone(), code, copy_in)
      .await?;

    file_map.write().unwrap().insert(self.copy_out.clone(), res);
    log::info!("compile for {} finished", &self.copy_out);

    return Ok(());
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerateCmd {
  pub lang: String,
  pub args: Vec<String>,
  pub exec: String,
  pub copy_in: HashMap<String, String>,
  pub copy_out: String,
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
    [self.copy_out.clone()].into()
  }

  async fn exec(
    &self,
    sandbox: &sandbox::Client,
    file_map: Arc<RwLock<HashMap<String, String>>>,
  ) -> Result<(), ExecuteError> {
    let lang = CONFIG
      .lang
      .get(&self.lang)
      .map_or(Err(ExecuteError::InvalidLang(self.lang.clone())), |x| Ok(x))?;
    let exec;
    let copy_in: HashMap<String, proto::File>;
    {
      let file_map = file_map.read().unwrap();
      exec = proto::File::Cached(file_map[&self.exec].clone().into());
      copy_in = self
        .copy_in
        .iter()
        .map(|f| {
          (
            f.0.to_string(),
            proto::File::Cached(file_map[f.1].clone().into()),
          )
        })
        .collect();
    }

    let res = sandbox
      .generate(lang, self.args.clone(), exec, copy_in)
      .await?;
    file_map.write().unwrap().insert(self.copy_out.clone(), res);

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
    sandbox: &sandbox::Client,
    file_map: Arc<RwLock<HashMap<String, String>>>,
  ) -> Result<(), ExecuteError> {
    let lang = CONFIG
      .lang
      .get(&self.lang)
      .map_or(Err(ExecuteError::InvalidLang(self.lang.clone())), |x| Ok(x))?;
    let exec;
    let inf;
    let copy_in: HashMap<String, proto::File>;
    {
      let file_map = file_map.read().unwrap();
      exec = proto::File::Cached(file_map[&self.exec].clone().into());
      inf = proto::File::Cached(file_map[&self.inf].clone().into());
      copy_in = self
        .copy_in
        .iter()
        .map(|f| {
          (
            f.0.to_string(),
            proto::File::Cached(file_map[f.1].clone().into()),
          )
        })
        .collect();
    }

    let overview = sandbox
      .validate(lang, self.args.clone(), exec, inf, copy_in)
      .await?;

    let report_id = sandbox
      .file_add(
        serde_json::to_string(&overview)
          .unwrap()
          .as_bytes()
          .to_vec(),
      )
      .await
      .map_or_else(
        |e| Err(ExecuteError::Runtime(Arc::new(e).into())),
        |x| Ok(x),
      )?;

    file_map
      .write()
      .unwrap()
      .insert(self.report.clone(), report_id);

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
    sandbox: &sandbox::Client,
    file_map: Arc<RwLock<HashMap<String, String>>>,
  ) -> Result<(), ExecuteError> {
    let lang = CONFIG
      .lang
      .get(&self.lang)
      .map_or(Err(ExecuteError::InvalidLang(self.lang.clone())), |x| Ok(x))?;
    let exec;
    let inf;
    let copy_in: HashMap<String, proto::File>;
    {
      let file_map = file_map.read().unwrap();
      exec = proto::File::Cached(file_map[&self.exec].clone().into());
      inf = proto::File::Cached(file_map[&self.inf].clone().into());
      copy_in = self
        .copy_in
        .iter()
        .map(|f| {
          (
            f.0.to_string(),
            proto::File::Cached(file_map[f.1].clone().into()),
          )
        })
        .collect();
    }

    let (res, copy_out) = sandbox
      .judge_batch(
        lang,
        self.args.clone(),
        exec,
        inf,
        copy_in,
        self.time_limit,
        self.memory_limit,
      )
      .await;

    if res.status != result::Status::Accepted {
      return Err(ExecuteError::Runtime(res.into()));
    }

    file_map
      .write()
      .unwrap()
      .insert(self.copy_out.clone(), copy_out.unwrap());
    return Ok(());
  }
}
