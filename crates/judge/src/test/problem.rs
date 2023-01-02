use std::{collections::HashMap, str::FromStr, sync::Arc, time};

use crate::{builtin, file, generator, lang, problem, program, result, sandbox};

#[test]
fn test_judge_a_plus_b() {
  super::async_test(async {
    let sol_c = program::Source {
      lang: lang::Lang::from_str("c").unwrap(),
      data: file::File::Memory(
        "
        #include<stdio.h>
        int main(){int a,b;scanf(\"%d%d\",&a,&b);printf(\"%d\\n\",a+b);}
        "
        .as_bytes()
        .to_vec(),
      ),
    };

    let sol_cpp = program::Source {
      lang: lang::Lang::from_str("cpp").unwrap(),
      data: file::File::Memory(
        "
        #include <iostream>
        signed main(){
          int a,b;
          std::cin>>a>>b;
          std::cout<<a+b<<std::endl;
        }
        "
        .as_bytes()
        .to_vec(),
      ),
    };

    let subtask = problem::Subtask {
      id: 1,
      score: 100.,
      dependences: vec![],
      testset: problem::Testset::Main,
      tests: vec![
        problem::Test {
          input: problem::Input::Plain {
            context: "12 34\n".as_bytes().to_vec(),
          },
          answer: problem::Answer::Generated,
        },
        problem::Test {
          input: problem::Input::Generated {
            generator: generator::Generator::from(
              program::Source {
                lang: lang::Lang::from_str("cpp").unwrap(),
                data: file::File::Memory(
                  "
                  #include\"testlib.h\"
                  #include<iostream>
                  signed main(signed argc,char**argv){
                    registerGen(argc,argv,1);
                    int n=opt<int>(\"n\");
                    std::cout<<rnd.next(0,n)<<' '<<rnd.next(0,n)<<'\\n';
                  }
                  "
                  .as_bytes()
                  .to_vec(),
                ),
              }
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
            ),
            args: vec!["-n".to_string(), "100".to_string()],
          },
          answer: problem::Answer::Generated,
        },
      ],
      time_limit: time::Duration::from_secs(1),
      memory_limit: 64 * 1024 * 1024,
    };

    let chk = program::Source {
      lang: lang::Lang::from_str("cpp").unwrap(),
      data: builtin::File::from_str("checker:ncmp.cpp").unwrap().into(),
    };

    let user_copy_in = HashMap::from([(
      "testlib.h".to_string(),
      Arc::new(
        sandbox::FileHandle::upload(
          builtin::File::from_str("testlib:testlib.h")
            .unwrap()
            .as_bytes(),
        )
        .await,
      ),
    )]);

    let (score, records) = subtask
      .judge(
        &sol_c.compile(vec![], user_copy_in.clone()).await.unwrap(),
        &sol_cpp.compile(vec![], user_copy_in.clone()).await.unwrap(),
        &chk
          .compile(vec![], user_copy_in.clone())
          .await
          .unwrap()
          .into(),
        &user_copy_in,
        &HashMap::new(),
        None,
      )
      .await;

    assert_eq!(score, 1.);
    for record in &records {
      assert_eq!(record.status, result::RecordStatus::Accepted);
    }
  });
}
