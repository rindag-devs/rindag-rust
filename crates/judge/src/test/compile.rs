use std::collections::HashMap;

use crate::{
  result,
  sandbox::{self, proto},
  test, CONFIG,
};

async fn compile_test_c_prog(sandbox: &sandbox::Client) -> Result<String, result::Error> {
  sandbox
    .compile(
      &CONFIG.lang["c"],
      vec![],
      proto::File::Memory(
        "#include\"my_head.c\"\nint main(){puts(\"hello\");func();return 0;}".into(),
      ),
      [(
        "my_head.c".to_string(),
        proto::File::Memory(
          "#include<stdio.h>\nvoid func(){int x;scanf(\"%d\",&x);printf(\"func: %d\\n\",x);}"
            .into(),
        ),
      )]
      .into(),
    )
    .await
}

#[tokio::test]
async fn test_ce() {
  test::init();

  let sandbox = sandbox::Client::from_global_config().await;
  let res = sandbox
    .compile(
      &CONFIG.lang["c"],
      vec![],
      proto::File::Memory("ERROR!".into()),
      HashMap::new(),
    )
    .await;

  assert!(res.is_err());
}

#[tokio::test]
async fn test_ok() {
  test::init();

  let sandbox = sandbox::Client::from_global_config().await;
  let exec_id = compile_test_c_prog(&sandbox).await.unwrap();

  let res = sandbox
    .judge_batch(
      &CONFIG.lang["c"],
      [].into(),
      proto::File::Cached(exec_id.into()),
      proto::File::Memory("998244343".into()),
      HashMap::new(),
    )
    .await;

  assert_eq!(res.0.status, result::Status::Accepted);

  let output = sandbox.file_get(res.1.unwrap()).await.unwrap();

  assert_eq!(
    output.content,
    "hello\nfunc: 998244343\n".as_bytes().to_vec()
  );
}
