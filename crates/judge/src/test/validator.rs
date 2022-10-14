use std::collections::HashMap;

use crate::{
  builtin,
  sandbox::{self, proto},
  test, validator, CONFIG,
};

#[tokio::test]
async fn test_val_a_plus_b() {
  test::init();

  let sandbox = sandbox::Client::from_global_config().await;

  let exec_id = sandbox
    .compile(
      &CONFIG.lang["cpp"],
      vec![],
      proto::File::Memory(
        "
        #include\"testlib.h\"
        signed main(signed argc,char**argv){
          registerValidation(argc,argv);
          int a=inf.readInt(-100,100,\"a\");
          inf.readSpace();
          int b=inf.readInt(-100,100,\"b\");
          if (validator.group() == \"even_a_and_b\") {
            ensure(a % 2 == 0);
            ensure(b % 2 == 0);
          }
          inf.readEoln();
          inf.readEof();
          addFeature(\"sum_0\");
          if(a+b==0)feature(\"sum_0\");
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

  assert_eq!(
    sandbox
      .validate(
        &CONFIG.lang["cpp"],
        vec!["--group".to_string(), "even_a_and_b".to_string()],
        proto::File::Cached(exec_id.clone().into()),
        proto::File::Memory("0 -10\n".into()),
        HashMap::new(),
      )
      .await
      .unwrap(),
    validator::Overview {
      variables: [
        (
          "a".to_string(),
          validator::VariableBounds {
            hit_min: false,
            hit_max: false
          }
        ),
        (
          "b".to_string(),
          validator::VariableBounds {
            hit_min: false,
            hit_max: false
          }
        ),
      ]
      .into(),
      features: [("sum_0".to_string(), false)].into(),
    }
  );

  assert_eq!(
    sandbox
      .validate(
        &CONFIG.lang["cpp"],
        vec![],
        proto::File::Cached(exec_id.clone().into()),
        proto::File::Memory("-100 100\n".into()),
        HashMap::new(),
      )
      .await
      .unwrap(),
    validator::Overview {
      variables: [
        (
          "a".to_string(),
          validator::VariableBounds {
            hit_min: true,
            hit_max: false
          }
        ),
        (
          "b".to_string(),
          validator::VariableBounds {
            hit_min: false,
            hit_max: true
          }
        ),
      ]
      .into(),
      features: [("sum_0".to_string(), true)].into(),
    }
  );

  assert!(sandbox
    .validate(
      &CONFIG.lang["cpp"],
      vec![],
      proto::File::Cached(exec_id.clone().into()),
      proto::File::Memory("-100 101\n".into()),
      HashMap::new(),
    )
    .await
    .is_err());

  assert!(sandbox
    .validate(
      &CONFIG.lang["cpp"],
      vec!["--group".to_string(), "even_a_and_b".to_string()],
      proto::File::Cached(exec_id.clone().into()),
      proto::File::Memory("1 2\n".into()),
      HashMap::new(),
    )
    .await
    .is_err());
}
