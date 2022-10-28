use std::{collections::HashMap, str::FromStr, sync::Arc, time};

use crate::{compile, etc, judge, sandbox};

#[test]
fn test_ce() {
  super::test_rt().block_on(async {
    super::init();

    let res = compile::compile(
      &etc::LangCfg::from_str("c").unwrap(),
      vec![],
      Arc::new(sandbox::FileHandle::upload("ERROR!".as_bytes()).await),
      HashMap::new(),
    )
    .await;

    assert!(res.is_err());
  });
}

#[test]
fn test_ok() {
  super::test_rt().block_on(async {
    super::init();

    let exec_file = compile::compile(
      &etc::LangCfg::from_str("c").unwrap(),
      vec![],
      Arc::new(
        sandbox::FileHandle::upload(
          "#include\"my_head.c\"\nint main(){puts(\"hello\");func();return 0;}".as_bytes(),
        )
        .await,
      ),
      [(
        "my_head.c".to_string(),
        Arc::new(
          sandbox::FileHandle::upload(
            "#include<stdio.h>\nvoid func(){int x;scanf(\"%d\",&x);printf(\"func: %d\\n\",x);}"
              .as_bytes(),
          )
          .await,
        ),
      )]
      .into(),
    )
    .await
    .unwrap();

    let res = judge::judge_batch(
      &etc::LangCfg::from_str("c").unwrap(),
      [].into(),
      exec_file.clone(),
      Arc::new(sandbox::FileHandle::upload("998244353".as_bytes()).await),
      [].into(),
      time::Duration::from_secs(1),
      64 * 1024 * 1024,
    )
    .await;

    assert_eq!(res.0.status, sandbox::Status::Accepted);

    assert_eq!(
      res.1.unwrap().to_vec().await.unwrap(),
      "hello\nfunc: 998244353\n".as_bytes().to_vec()
    );
  });
}
