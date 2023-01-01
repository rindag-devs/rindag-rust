use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::{builtin, file, lang, program, sandbox, validator};

#[test]
fn test_val_a_plus_b() {
  super::test_rt().block_on(async {
    super::init();

    let src = program::Source {
      lang: lang::Lang::from_str("cpp").unwrap(),
      data: file::File::Memory(
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
        .as_bytes()
        .to_vec(),
      ),
    };

    let val = validator::Validator::from(
      src
        .compile(
          vec![],
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
        .unwrap(),
    );

    assert_eq!(
      val
        .validate(
          vec!["--group".to_string(), "even_a_and_b".to_string()],
          Arc::new(sandbox::FileHandle::upload("0 -10\n".as_bytes()).await),
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
      val
        .validate(
          vec![],
          Arc::new(sandbox::FileHandle::upload("-100 100\n".as_bytes()).await),
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

    assert!(val
      .validate(
        vec![],
        Arc::new(sandbox::FileHandle::upload("-100 101\n".as_bytes()).await),
        HashMap::new(),
      )
      .await
      .is_err());

    assert!(val
      .validate(
        vec!["--group".to_string(), "even_a_and_b".to_string()],
        Arc::new(sandbox::FileHandle::upload("1 2\n".as_bytes()).await),
        HashMap::new(),
      )
      .await
      .is_err());
  });
}
