use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::{builtin, data, generator, lang, program, sandbox};

#[test]
fn test_simple() {
  super::async_test(async {
    let src = program::Source {
      lang: lang::Lang::from_str("cpp").unwrap(),
      data: data::Provider::Memory(
        "
        #include\"testlib.h\"
        #include<iostream>
        signed main(signed argc,char**argv){
          registerGen(argc,argv,1);
          int n=opt<int>(\"n\");
          std::cout<<n<<'\\n';
        }
        "
        .as_bytes()
        .to_vec(),
      ),
    };

    let gen = generator::Generator::from(
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
      gen
        .generate(vec!["-n".to_string(), "100".to_string()], HashMap::new(),)
        .await
        .unwrap()
        .context()
        .await
        .unwrap(),
      "100\n".as_bytes()
    );
  });
}
