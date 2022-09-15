use core::time;
use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;

use crate::service::judge::{self, exec};

fn init() {
  let _ = pretty_env_logger::env_logger::Builder::from_env(
    pretty_env_logger::env_logger::Env::default().default_filter_or("info"),
  )
  .is_test(true)
  .try_init();
}

/// A test for judge running `/usr/bin/cat` to print the file content.
///
/// This test uses raw websocket connection without a go-judge client.
#[tokio::test]
async fn test_judge_raw_cat() {
  init();

  let (socket, _) =
    tokio_tungstenite::connect_async(url::Url::parse("ws://localhost:5050/ws").unwrap())
      .await
      .expect("Can't connect");

  let (mut write, mut read) = socket.split();

  let req_id = uuid::Uuid::new_v4();

  let req = exec::Request {
    request_id: req_id.clone(),
    cmd: vec![exec::Cmd {
      args: vec!["/usr/bin/cat".to_string(), "a.txt".to_string()],
      files: vec![
        exec::File::Memory {
          content: "".to_string(),
        },
        exec::File::Collector {
          name: "stdout".to_string(),
          max: 10240,
          pipe: false,
        },
      ],
      copy_in: HashMap::from([(
        "a.txt".to_string(),
        exec::File::Memory {
          content: "\x01na\u{00ef}ve\n".to_string(),
        },
      )]),
      cpu_limit: std::time::Duration::from_secs(200).as_nanos() as u64,
      clock_limit: std::time::Duration::from_secs(200).as_nanos() as u64,
      copy_out: vec!["stdout".to_string()],
      ..Default::default()
    }],
    pipe_mapping: vec![],
  };

  // dbg!(serde_json::to_string(&req).unwrap());

  write
    .send(Message::Text(serde_json::to_string(&req).unwrap()))
    .await
    .unwrap();

  sleep(time::Duration::from_secs(1)).await;

  loop {
    match read.next().await.unwrap().unwrap() {
      Message::Text(s) => {
        let resp: exec::WSResult = serde_json::from_str(&s).unwrap();

        assert_eq!(resp.request_id, req_id);
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0].status, exec::Status::Accepted);
        assert_eq!(resp.results[0].exit_status, 0);
        assert_eq!(resp.results[0].files["stdout"], "\x01naïve\n");

        break;
      }
      _ => {}
    }
  }

  write.close().await.unwrap();
}

/// A test for judge compiling and running a C code with gcc.
#[tokio::test]
async fn test_judge_gcc() {
  init();

  let mut client = judge::Client::new("localhost:5050", false).await;

  let (_, rx) = client
    .run(
      vec![exec::Cmd {
        args: vec!["/usr/bin/gcc".to_string(), "a.c".to_string()],
        copy_in: HashMap::from([(
          "a.c".to_string(),
          exec::File::Memory {
            content: "#include<stdio.h>\nint main(){puts(\"hello\");}".to_string(),
          },
        )]),
        copy_out: vec![],
        copy_out_cached: vec!["a.out".to_string()],
        ..Default::default()
      }],
      vec![],
    )
    .await
    .unwrap();

  let compile_res = &rx.await.unwrap().results[0];

  assert_eq!(compile_res.status, exec::Status::Accepted);

  let exec_file = compile_res.file_ids["a.out"].to_string();

  // dbg!(&exec_file);

  let (_, rx) = client
    .run(
      vec![exec::Cmd {
        args: vec!["a.out".to_string()],
        copy_in: HashMap::from([(
          "a.out".to_string(),
          exec::File::Prepared { file_id: exec_file },
        )]),
        ..Default::default()
      }],
      vec![],
    )
    .await
    .unwrap();

  let run_res = &rx.await.unwrap().results[0];

  assert_eq!(run_res.status, exec::Status::Accepted);
  assert_eq!(run_res.files["stdout"], "hello\n");
}
