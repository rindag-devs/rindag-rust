use std::{
  collections::HashMap,
  str::FromStr,
  sync::{Arc, Mutex},
};

use bytes::Bytes;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::oneshot};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

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
  pub async fn new(host: &str, security: bool) -> Self {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let senders = Arc::new(Mutex::new(HashMap::<
      uuid::Uuid,
      oneshot::Sender<exec::WSResult>,
    >::new()));
    let http_host =
      url::Url::from_str(&(if security { "https://" } else { "http://" }.to_string() + host))
        .expect("Invalid url");
    let ws_socket = tokio_tungstenite::connect_async(
      url::Url::parse(&(if security { "wss://" } else { "ws://" }.to_string() + host + "/ws"))
        .unwrap(),
    )
    .await
    .expect(&format!("Failed to connect to websocket {}", host))
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
  ) -> Result<(uuid::Uuid, oneshot::Receiver<WSResult>), tokio_tungstenite::tungstenite::Error> {
    let req = exec::Request::new(cmd, pipe_mapping);

    let (tx, rx) = oneshot::channel();
    let _ = self
      .senders
      .lock()
      .unwrap()
      .insert(req.request_id.clone(), tx);

    self
      .ws_writer
      .send(Message::Text(serde_json::to_string(&req).unwrap()))
      .await?;

    return Ok((req.request_id, rx));
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
