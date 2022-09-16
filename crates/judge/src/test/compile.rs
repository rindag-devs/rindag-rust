use std::collections::HashMap;

use crate::{etc::CONFIG, sandbox::exec, task};

#[tokio::test]
async fn test_compile_c_ok() {
  let res = task::compile(
    &CONFIG.read().unwrap().lang["c"],
    exec::File::Memory {
      content: "#include\"my_head.h\"\nint main(){puts(\"hello\");func();}".to_string(),
    },
    HashMap::from([(
      "my_head.h".to_string(),
      exec::File::Memory {
        content: "#include<stdio.h>\nvoid func(){puts(\"func\");}".to_string(),
      },
    )]),
  )
  .await;

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
