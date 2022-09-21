use std::collections::HashMap;

use crate::{
  sandbox::{self, proto},
  test,
};

/// A test for sandbox running `/usr/bin/cat` to print the file content.
///
/// This test uses raw gRpc connection without a go-judge client.
#[tokio::test]
async fn test_raw_cat() {
  test::init();

  let mut client = proto::ExecutorClient::connect(tonic::transport::Channel::from_static(
    "http://localhost:5051",
  ))
  .await
  .unwrap();

  let content = vec![9, 9, 8, 2, 100, 200, 240, 255];

  let req = proto::Request {
    cmd: vec![proto::request::CmdType {
      args: vec!["/usr/bin/cat".to_string(), "a.txt".to_string()],
      files: vec![
        proto::request::File {
          file: Some(proto::File::Memory(vec![].into())),
        },
        proto::request::File {
          file: Some(proto::File::Pipe(proto::PipeCollector {
            name: "stdout".to_string(),
            max: 10240,
            pipe: false,
          })),
        },
      ],
      copy_in: HashMap::from([(
        "a.txt".to_string(),
        proto::request::File {
          file: Some(proto::File::Memory(content.clone().into())),
        },
      )]),
      cpu_time_limit: std::time::Duration::from_secs(200).as_nanos() as u64,
      clock_time_limit: std::time::Duration::from_secs(200).as_nanos() as u64,
      memory_limit: 1024 * 1024 * 1024,
      proc_limit: 16,
      copy_out: vec![proto::request::CmdCopyOutFile {
        name: "stdout".to_string(),
        optional: false,
      }],
      ..Default::default()
    }],
    pipe_mapping: vec![],
    ..Default::default()
  };

  let resp = client.exec(req).await.unwrap().get_ref().clone();

  assert_eq!(resp.results.len(), 1);
  assert_eq!(resp.results[0].status(), proto::StatusType::Accepted);
  assert_eq!(resp.results[0].exit_status, 0);
  assert_eq!(resp.results[0].files["stdout"], content);
}

/// A test for sandbox compiling and running a C code with gcc.
#[tokio::test]
async fn test_hello_world() {
  test::init();

  let sandbox = sandbox::Client::from_global_config().await;

  let compile_res = &sandbox
    .exec(
      vec![proto::Cmd {
        args: vec!["/usr/bin/gcc".to_string(), "a.c".to_string()],
        copy_in: [(
          "a.c".to_string(),
          proto::File::Memory(
            "#include<stdio.h>\nint main(){puts(\"hello, world!\\n你好, 世界!\");}".into(),
          ),
        )]
        .into(),
        copy_out_cached: vec!["a.out".to_string()],
        ..Default::default()
      }],
      vec![],
    )
    .await
    .unwrap()
    .results[0];

  assert_eq!(compile_res.status(), proto::StatusType::Accepted);

  let exec_id = compile_res.file_ids["a.out"].to_string();

  // dbg!(&exec_file);

  let run_res = &sandbox
    .exec(
      vec![proto::Cmd {
        args: vec!["a.out".to_string()],
        copy_in: [("a.out".to_string(), proto::File::Cached(exec_id.into()))].into(),
        copy_out: vec!["stdout".to_string()],
        ..Default::default()
      }],
      vec![],
    )
    .await
    .unwrap()
    .results[0];

  assert_eq!(run_res.status(), proto::StatusType::Accepted);
  assert_eq!(
    run_res.files["stdout"],
    "hello, world!\n你好, 世界!\n".as_bytes().to_vec()
  );
}
