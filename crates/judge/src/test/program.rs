use std::{collections::HashMap, str::FromStr, time};

use crate::{data, lang, program, sandbox};

#[test]
fn test_ce() {
  super::async_test(async {
    let src = program::Source {
      lang: lang::Lang::from_str("c").unwrap(),
      data: data::Provider::Memory("ERROR".as_bytes().to_vec()),
    };

    let res = src.compile(vec![], HashMap::new()).await;

    assert!(res.is_err());
  });
}

#[test]
fn test_ok() {
  super::async_test(async {
    let src = program::Source {
      lang: lang::Lang::from_str("c").unwrap(),
      data: data::Provider::Memory(
        "#include\"my_head.c\"\nint main(){puts(\"hello\");func();return 0;}"
          .as_bytes()
          .to_vec(),
      ),
    };

    let exec = src
      .compile(
        vec![],
        [(
          "my_head.c".to_string(),
          sandbox::FileHandle::upload(
            "#include<stdio.h>\nvoid func(){int x;scanf(\"%d\",&x);printf(\"func: %d\\n\",x);}"
              .as_bytes(),
          )
          .await,
        )]
        .into(),
      )
      .await
      .unwrap();

    let res = exec
      .judge_batch(
        vec![],
        sandbox::FileHandle::upload("998244353".as_bytes()).await,
        [].into(),
        time::Duration::from_secs(1),
        64 * 1024 * 1024,
      )
      .await;

    assert_eq!(res.0.status, sandbox::Status::Accepted);

    assert_eq!(
      res.1.unwrap().context().await.unwrap(),
      "hello\nfunc: 998244353\n".as_bytes().to_vec()
    );
  });
}
