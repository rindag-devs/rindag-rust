use std::collections::HashMap;

use crate::{etc::CONFIG, result, sandbox::proto, task, CLIENT};

async fn compile_test_c_prog() -> (result::CompileResult, Result<String, proto::FileError>) {
  return task::compile(
    &CONFIG.lang["c"],
    proto::File::Memory(proto::MemoryFile {
      content: "#include\"my_head.c\"\nint main(){puts(\"hello\");func();return 0;}"
        .as_bytes()
        .to_vec(),
    }),
    HashMap::from([(
      "my_head.c".to_string(),
      proto::File::Memory(proto::MemoryFile {
        content:
          "#include<stdio.h>\nvoid func(){int x;scanf(\"%d\",&x);printf(\"func: %d\\n\",x);}"
            .as_bytes()
            .to_vec(),
      }),
    )]),
  )
  .await;
}

#[tokio::test]
async fn test_compile_c_ok() {
  let res = compile_test_c_prog().await;

  assert_eq!(res.0.status, proto::StatusType::Accepted);
  assert!(res.1.is_ok());
}

#[tokio::test]
async fn test_compile_c_ce() {
  let res = task::compile(
    &CONFIG.lang["c"],
    proto::File::Memory(proto::MemoryFile {
      content: "ERROR!".as_bytes().to_vec(),
    }),
    HashMap::new(),
  )
  .await;

  assert_eq!(res.0.status, proto::StatusType::NonZeroExitStatus);
  assert!(res.1.is_err());
}

#[tokio::test]
async fn test_compile_run_batch() {
  let res = compile_test_c_prog().await;

  assert_eq!(res.0.status, proto::StatusType::Accepted);
  assert!(res.1.is_ok());

  let exec_id = res.1.unwrap();

  let res = task::judge_batch(
    &CONFIG.lang["c"],
    proto::File::Cached(proto::CachedFile { file_id: exec_id }),
    proto::File::Memory(proto::MemoryFile {
      content: "998244343".as_bytes().to_vec(),
    }),
    HashMap::new(),
  )
  .await;

  assert_eq!(res.0.status, result::Status::Accepted);
  assert!(res.1.is_ok());

  let output = CLIENT
    .get()
    .await
    .as_ref()
    .file_get(res.1.unwrap())
    .await
    .unwrap();

  assert_eq!(
    output.content,
    "hello\nfunc: 998244343\n".as_bytes().to_vec()
  );
}
