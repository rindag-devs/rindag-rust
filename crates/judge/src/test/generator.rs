use std::collections::HashMap;

use crate::{
  builtin,
  etc::CONFIG,
  sandbox::{self, proto},
  test,
};

#[tokio::test]
async fn test_simple() {
  test::init();

  let sandbox = sandbox::Client::from_global_config().await;

  let exec_id = sandbox
    .compile(
      &CONFIG.lang["cpp"],
      vec![],
      proto::File::Memory(
        "
        #include\"testlib.h\"
        #include<iostream>
        signed main(signed argc,char**argv){
          registerGen(argc,argv,1);
          int n=opt<int>(\"n\");
          std::cout<<n<<'\\n';
        }
        "
        .into(),
      ),
      [(
        "testlib.h".to_string(),
        proto::File::Memory(
          builtin::Testlib::get("testlib.h")
            .unwrap()
            .data
            .to_vec()
            .into(),
        ),
      )]
      .into(),
    )
    .await
    .unwrap();

  let file_id = sandbox
    .generate(
      &CONFIG.lang["cpp"],
      vec!["-n".to_string(), "100".to_string()],
      proto::File::Cached(exec_id.into()),
      HashMap::new(),
    )
    .await
    .unwrap();

  assert_eq!(
    sandbox.file_get(file_id).await.unwrap().content,
    "100\n".as_bytes()
  );
}
