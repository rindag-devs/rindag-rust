use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::{builtin, compile, etc, generator, sandbox};

#[test]
fn test_simple() {
  super::test_rt().block_on(async {
    super::init();

    let exec_file = compile::compile(
      &etc::LangCfg::from_str("cpp").unwrap(),
      vec![],
      Arc::new(
        sandbox::FileHandle::upload(
          "
        #include\"testlib.h\"
        #include<iostream>
        signed main(signed argc,char**argv){
          registerGen(argc,argv,1);
          int n=opt<int>(\"n\");
          std::cout<<n<<'\\n';
        }
        "
          .as_bytes(),
        )
        .await,
      ),
      [(
        "testlib.h".to_string(),
        Arc::new(
          sandbox::FileHandle::upload(
            &builtin::File::from_str("testlib:testlib.h")
              .unwrap()
              .as_bytes(),
          )
          .await,
        ),
      )]
      .into(),
    )
    .await
    .unwrap();

    assert_eq!(
      generator::generate(
        &etc::LangCfg::from_str("cpp").unwrap(),
        vec!["-n".to_string(), "100".to_string()],
        exec_file,
        HashMap::new(),
      )
      .await
      .unwrap()
      .to_vec()
      .await
      .unwrap(),
      "100\n".as_bytes()
    );
  });
}
