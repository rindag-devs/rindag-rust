use std::{collections::HashMap, str::FromStr, time};

use crate::{
  etc, result,
  sandbox::{self, proto},
  test,
};

async fn compile_test_c_prog(sandbox: &sandbox::Client) -> Result<String, result::Error> {
  sandbox
    .compile(
      &etc::LangCfg::from_str("c").unwrap(),
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
      &etc::LangCfg::from_str("c").unwrap(),
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
      &etc::LangCfg::from_str("c").unwrap(),
      [].into(),
      proto::File::Cached(exec_id.into()),
      proto::File::Memory("998244353".into()),
      HashMap::new(),
      time::Duration::from_secs(1),
      64 * 1024 * 1024,
    )
    .await;

  assert_eq!(res.0.status, result::ExecuteStatus::Accepted);

  let output = sandbox.file_get(res.1.unwrap()).await.unwrap();

  assert_eq!(
    output.content,
    "hello\nfunc: 998244353\n".as_bytes().to_vec()
  );
}
