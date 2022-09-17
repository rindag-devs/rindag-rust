use std::collections::HashMap;

use crate::{etc::CONFIG, result, sandbox::exec, task, CLIENT};

async fn compile_test_c_prog() -> (result::CompileResult, Result<String, exec::FileError>) {
  return task::compile(
    &CONFIG.read().unwrap().lang["c"],
    exec::File::Memory {
      content: "#include\"my_head.c\"\nint main(){puts(\"hello\");func();return 0;}".to_string(),
    },
    HashMap::from([(
      "my_head.c".to_string(),
      exec::File::Memory {
        content:
          "#include<stdio.h>\nvoid func(){int x;scanf(\"%d\",&x);printf(\"func: %d\\n\",x);}"
            .to_string(),
      },
    )]),
  )
  .await;
}

#[tokio::test]
async fn test_compile_c_ok() {
  let res = compile_test_c_prog().await;

  assert_eq!(res.0.status, exec::Status::Accepted);
  assert!(res.1.is_ok());
}

#[tokio::test]
async fn test_compile_c_ce() {
  let res = task::compile(
    &CONFIG.read().unwrap().lang["c"],
    exec::File::Memory {
      content: "ERROR!".to_string(),
    },
    HashMap::new(),
  )
  .await;

  assert_eq!(res.0.status, exec::Status::NonzeroExitStatus);
  assert!(res.1.is_err());
}

#[tokio::test]
async fn test_compile_run_batch() {
  let res = compile_test_c_prog().await;

  assert_eq!(res.0.status, exec::Status::Accepted);
  assert!(res.1.is_ok());

  let exec_id = res.1.unwrap();

  let res = task::judge_batch(
    &CONFIG.read().unwrap().lang["c"],
    exec::File::Prepared { file_id: exec_id },
    exec::File::Memory {
      content: "998244343".to_string(),
    },
    HashMap::new(),
  )
  .await;

  assert_eq!(res.0.status, result::Status::Accepted);
  assert!(res.1.is_ok());

  let output = CLIENT
    .get()
    .await
    .borrow()
    .get_file(&res.1.unwrap())
    .await
    .unwrap();

  assert_eq!(output, "hello\nfunc: 998244343\n");
}
