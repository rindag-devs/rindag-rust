use std::{str::FromStr, time};

use crate::{builtin, problem, result};

#[test]
fn test_judge_a_plus_b() {
  super::test_rt().block_on(async {
    super::init();

    let problem = problem::Problem {
      subtasks: vec![problem::Subtask {
        score: 100.,
        dependences: vec![],
        testset: problem::Testset::Main,
        tests: vec![
          problem::Test {
            input: "1 2\n".as_bytes().to_vec().into(),
            answer: "3\n".as_bytes().to_vec().into(),
          },
          problem::Test {
            input: "100 200\n".as_bytes().to_vec().into(),
            answer: "300\n".as_bytes().to_vec().into(),
          },
        ],
        time_limit: time::Duration::from_secs(1),
        memory_limit: 64 * 1024 * 1024,
      }],
      kind: problem::Kind::Batch,
      checker: problem::SourceCode {
        lang: "cpp".to_string(),
        data: builtin::File::from_str("checker:ncmp.cpp").unwrap().into(),
      },
      user_copy_in: [(
        "testlib.h".to_string(),
        builtin::File::from_str("testlib:testlib.h").unwrap().into(),
      )]
      .into(),
      judge_copy_in: [].into(),
    };

    let result = problem
      .judge(problem::SourceCode {
        lang: "cpp".to_string(),
        data: "
      #include<iostream>
      using namespace std;
      signed main(){
        int a,b;cin>>a>>b;
        cout<<a+b<<'\\n';
      }
      "
        .as_bytes()
        .to_vec()
        .into(),
      })
      .await;

    let (score, _) = match result {
      result::JudgeResult::Ok { score, results } => (score, results),
      _ => panic!("excepted Ok, found {:?}", result),
    };

    assert_eq!(score, 100.);
  });
}
