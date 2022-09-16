use std::{
  cell::RefCell,
  collections::HashMap,
  rc::Rc,
  sync::{Arc, Mutex},
};

use async_once::AsyncOnce;
use bytes::Bytes;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::oneshot};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::etc::{self, CONFIG};

use super::exec::{self, WSResult};

/// go-judge client
pub struct Client {
  senders: Arc<Mutex<HashMap<uuid::Uuid, oneshot::Sender<exec::WSResult>>>>,
  http_host: url::Url,
  ws_writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
}

impl Client {
  /// Create a new client from host.
  ///
  /// If `security` is true, it will use wss and https.
  pub async fn new(http_host: url::Url, ws_host: &url::Url) -> Self {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let senders = Arc::new(Mutex::new(HashMap::<
      uuid::Uuid,
      oneshot::Sender<exec::WSResult>,
    >::new()));
    let ws_socket = tokio_tungstenite::connect_async(ws_host)
      .await
      .expect(&format!("Failed to connect to websocket {}", ws_host))
      .0;

    let (write, mut read) = ws_socket.split();

    {
      let senders = senders.clone();
      tokio::spawn(async move {
        while let Some(msg) = read.next().await {
          match msg {
            Ok(res) => {
              let senders = senders.clone();
              rt.spawn(async move {
                if let Message::Text(res) = res {
                  let res: exec::WSResult =
                    serde_json::from_str(&res).expect("WS socket result json parse error");
                  log::info!("Received request id: {}", res.request_id);
                  let _ = senders
                    .lock()
                    .unwrap()
                    .remove(&res.request_id)
                    .unwrap()
                    .send(res);
                }
              });
            }
            Err(e) => log::error!("Websocket read error: {}", e),
          }
        }
      });
    }

    return Client {
      http_host,
      senders,
      ws_writer: write,
    };
  }

  // Create a new client from global config.
  pub async fn from_config(cfg: &etc::Cfg) -> Self {
    Self::new(cfg.sandbox.http_host.clone(), &cfg.sandbox.ws_host).await
  }

  /// Get a file of sandbox server.
  ///
  /// It will return it's content.
  pub async fn get_file(&self, file_id: &str) -> Result<Bytes, reqwest::Error> {
    return Ok(
      reqwest::get(format!("{}/file/{}", &self.http_host, file_id))
        .await?
        .error_for_status()?
        .bytes()
        .await?,
    );
  }

  /// Prepare a file in the sandbox, returns file id (can be referenced in `run` parameter).
  pub async fn add_file(&self, content: Bytes) -> Result<String, reqwest::Error> {
    return Ok(
      reqwest::Client::new()
        .post(format!("{}/file", &self.http_host))
        .body(reqwest::Body::from(content))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?,
    );
  }

  /// Delete a file of sandbox server.
  pub async fn delete_file(&self, file_id: &str) -> Result<(), reqwest::Error> {
    reqwest::Client::new()
      .delete(format!("{}/file{}", &self.http_host, file_id))
      .send()
      .await?
      .error_for_status()?;
    return Ok(());
  }

  /// List all files of sandbox server.
  ///
  /// - Key of hashmap is file id.
  /// - Value of hashmap is file name.
  pub async fn list_files(&self) -> Result<HashMap<String, String>, reqwest::Error> {
    return Ok(
      reqwest::get(format!("{}/file", &self.http_host))
        .await?
        .error_for_status()?
        .json()
        .await?,
    );
  }

  /// Get go-judge server version.
  pub async fn version(&self) -> Result<String, reqwest::Error> {
    return Ok(
      reqwest::get(format!("{}/version", &self.http_host))
        .await?
        .error_for_status()?
        .text()
        .await?,
    );
  }

  /// Execute some command (then not wait).
  ///
  /// All the command will be executed parallelly.
  ///
  /// Returns the uuid of request and an oneshot result receiver.
  pub async fn run(
    &mut self,
    cmd: Vec<exec::Cmd>,
    pipe_mapping: Vec<exec::PipeMap>,
  ) -> (uuid::Uuid, oneshot::Receiver<WSResult>) {
    let req = exec::Request::new(cmd, pipe_mapping);

    let (tx, rx) = oneshot::channel();

    match self
      .ws_writer
      .send(Message::Text(serde_json::to_string(&req).unwrap()))
      .await
    {
      Ok(_) => {
        let _ = self
          .senders
          .lock()
          .unwrap()
          .insert(req.request_id.clone(), tx);
      }
      Err(e) => {
        log::error!("WebSocket send error: {}", e);
        let _ = tx.send(WSResult {
          request_id: req.request_id.clone(),
          results: vec![],
          error: Some(format!("WebSocket send error: {}", e)),
        });
      }
    }

    return (req.request_id, rx);
  }

  /// Cancel running a command.
  pub async fn cancel(
    &mut self,
    cancel_request_id: uuid::Uuid,
  ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let req = exec::CancelRequest { cancel_request_id };

    self
      .ws_writer
      .send(Message::Text(serde_json::to_string(&req).unwrap()))
      .await?;

    return Ok(());
  }
}

lazy_static! {
  pub static ref CLIENT: AsyncOnce<Rc<RefCell<Client>>> = AsyncOnce::new(async {
    return Rc::new(RefCell::new(
      Client::from_config(&CONFIG.read().unwrap()).await,
    ));
  });
}
