use std::{collections::HashMap, sync::Arc};

use async_once::AsyncOnce;
use tokio::sync::{oneshot, Semaphore};

use crate::etc::CONFIG;

use super::proto;

type ExecResult = Result<proto::Response, tonic::Status>;

/// go-judge client
pub struct Client {
  /// The gRpc client.
  client: proto::ExecutorClient<tonic::transport::Channel>,

  /// A semaphore to limit for max job count.
  semaphore: Arc<Semaphore>,

  /// Tokio runtime to spawn tasks.
  rt: tokio::runtime::Runtime,
}

impl Client {
  /// Create a new client from host.
  ///
  /// # Panics
  ///
  /// Panics if the endpoint connect error.
  pub async fn new(endpoint: tonic::transport::Endpoint, max_job: usize) -> Self {
    return Self {
      client: proto::ExecutorClient::connect(endpoint).await.unwrap(),
      semaphore: Arc::new(Semaphore::new(max_job)),
      rt: tokio::runtime::Builder::new_multi_thread().build().unwrap(),
    };
  }

  /// Get a file of sandbox server. and return it's content.
  ///
  /// # Errors
  ///
  /// This function will return an error if the file is not found or the connect is broken.
  pub async fn file_get(&self, file_id: String) -> Result<proto::FileContent, tonic::Status> {
    match self
      .client
      .clone()
      .file_get(proto::FileId { file_id })
      .await
    {
      Ok(res) => Ok(res.get_ref().clone()),
      Err(e) => Err(e),
    }
  }

  /// Prepare a file in the sandbox, returns file id (can be referenced in `run` parameter).
  pub async fn file_add(&self, content: Vec<u8>) -> Result<String, tonic::Status> {
    match self
      .client
      .clone()
      .file_add(proto::FileContent {
        content,
        ..Default::default()
      })
      .await
    {
      Ok(res) => Ok(res.get_ref().file_id.clone()),
      Err(e) => Err(e),
    }
  }

  /// Delete a file of sandbox server.
  pub async fn file_delete(&self, file_id: String) -> Result<(), tonic::Status> {
    match self
      .client
      .clone()
      .file_delete(proto::FileId { file_id })
      .await
    {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }

  /// List all files of sandbox server.
  ///
  /// - Key of hashmap is file id.
  /// - Value of hashmap is file name.
  pub async fn file_list(&self) -> Result<HashMap<String, String>, tonic::Status> {
    match self.client.clone().file_list(()).await {
      Ok(res) => Ok(res.get_ref().file_ids.clone()),
      Err(e) => Err(e),
    }
  }

  /// Execute some command (then not wait).
  ///
  /// All the command will be executed parallelly.
  ///
  /// Returns the uuid of request and an oneshot result receiver.
  pub async fn exec(
    &self,
    cmd: Vec<proto::Cmd>,
    pipe_mapping: Vec<proto::PipeMap>,
  ) -> oneshot::Receiver<ExecResult> {
    let req = proto::Request {
      cmd: cmd.into_iter().map(|c| c.into()).collect(),
      pipe_mapping,
      ..Default::default()
    };

    let (tx, rx) = oneshot::channel();

    let client = self.client.clone();
    let permit = self.semaphore.clone().acquire_owned().await.unwrap();

    self.rt.spawn(async move {
      let _ = tx.send(match client.clone().exec(req).await {
        Ok(res) => Ok(res.get_ref().clone()),
        Err(e) => Err(e),
      });
      drop(permit);
    });

    return rx;
  }
}

lazy_static! {
  pub static ref CLIENT: AsyncOnce<Arc<Client>> = AsyncOnce::new(async {
    let conf = &CONFIG.sandbox;
    return Arc::new(
      Client::new(
        tonic::transport::Channel::from_static(&conf.host),
        conf.max_job,
      )
      .await,
    );
  });
}
