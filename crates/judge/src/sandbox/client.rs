use std::collections::HashMap;

use async_once::AsyncOnce;
use thiserror::Error;

use crate::{etc, sandbox::proto, CONFIG};

/// go-judge client
#[derive(Clone)]
pub struct Client {
  /// The gRPC client.
  client: proto::executor_client::ExecutorClient<tonic::transport::Channel>,
}

impl Client {
  /// Create a new client from host.
  ///
  /// # Panics
  ///
  /// Panics if the endpoint connect error.
  async fn connect(conf: &etc::SandboxCfg) -> Self {
    return Self {
      client: proto::executor_client::ExecutorClient::connect(conf.host.clone())
        .await
        .unwrap(),
    };
  }

  /// Get a file of sandbox server. and return it's content.
  ///
  /// # Errors
  ///
  /// This function will return an error if the file is not found or the connect is broken.
  pub(super) async fn file_get(&self, file_id: &str) -> Result<Vec<u8>, FileGetError> {
    match self
      .client
      .clone()
      .file_get(proto::FileId {
        file_id: file_id.to_string(),
      })
      .await
    {
      Ok(f) => Ok(f.get_ref().content.clone()),
      Err(err) => match err.code() {
        tonic::Code::NotFound => Err(FileGetError {
          id: file_id.to_string(),
        }),
        _ => panic!("file get error: {}", err),
      },
    }
  }

  /// Prepare a file in the sandbox, returns file id (can be referenced in `run` parameter).
  pub(super) async fn file_add(&self, content: &[u8]) -> String {
    self
      .client
      .clone()
      .file_add(proto::FileContent {
        content: content.to_vec(),
        ..Default::default()
      })
      .await
      .unwrap()
      .get_ref()
      .file_id
      .clone()
  }

  /// Delete a file of sandbox server.
  pub(super) async fn file_delete(&self, file_id: &str) {
    self
      .client
      .clone()
      .file_delete(proto::FileId {
        file_id: file_id.to_string(),
      })
      .await
      .unwrap();
  }

  /// List all files of sandbox server.
  ///
  /// - Key of hashmap is file id.
  /// - Value of hashmap is file name.
  #[allow(dead_code)]
  pub async fn file_list(&self) -> HashMap<String, String> {
    self
      .client
      .clone()
      .file_list(())
      .await
      .unwrap()
      .get_ref()
      .file_ids
      .clone()
  }

  /// Execute some command (then not wait).
  ///
  /// All the command will be executed parallelly.
  ///
  /// Returns the uuid of request and an oneshot result receiver.
  pub(super) async fn exec(&self, req: proto::Request) -> proto::Response {
    let client = self.client.clone();
    let res = client.clone().exec(req).await.unwrap();
    res.get_ref().clone()
  }
}

#[derive(Debug, Error)]
#[error("file get error: {id}")]
pub struct FileGetError {
  pub id: String,
}

lazy_static! {
  pub(super) static ref CLIENT: AsyncOnce<Client> =
    AsyncOnce::new(async { Client::connect(&CONFIG.sandbox).await });
}
