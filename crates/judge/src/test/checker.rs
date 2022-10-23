use std::{collections::HashMap, str::FromStr};

use crate::{
  builtin,
  checker::{self, Output},
  etc,
  sandbox::{self, proto},
};

#[test]
fn test_parse_output() {
  assert_eq!(
    Output::parse("ok you win\n3 steps."),
    Output {
      status: checker::Status::Accepted,
      score: 1.0f32,
      message: "ok you win\n3 steps.".to_string()
    }
  );

  assert_eq!(
    Output::parse("wrong answer you lose\n12 steps."),
    Output {
      status: checker::Status::WrongAnswer,
      score: 0.0f32,
      message: "wrong answer you lose\n12 steps.".to_string()
    }
  );

  assert_eq!(
    Output::parse("points 0.12 you used 12 / 100 moves"),
    Output {
      status: checker::Status::PartiallyCorrect,
      score: 0.12f32,
      message: "points 0.12 you used 12 / 100 moves".to_string()
    }
  );

  assert_eq!(
    Output::parse("wrong output format \t \textra spaces\n\t\t"),
    Output {
      status: checker::Status::PresentationError,
      score: 0.0f32,
      message: "wrong output format \t \textra spaces\n\t\t".to_string()
    }
  );

  assert_eq!(
    Output::parse("status(accepted)\nscore(0.1)"),
    Output {
      status: checker::Status::Accepted,
      score: 0.1f32,
      message: "status(accepted)\nscore(0.1)".to_string()
    }
  );
}

#[tokio::test]
async fn test_builtin_checker() {
  let sandbox = sandbox::Client::from_global_config().await;
  let checker = proto::File::Memory(
    builtin::Checker::get("ncmp.cpp")
      .unwrap()
      .data
      .to_vec()
      .into(),
  );

  let exec_id = sandbox
    .compile(
      &etc::LangCfg::from_str("cpp").unwrap(),
      vec![],
      checker,
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

  let res = sandbox
    .check(
      &etc::LangCfg::from_str("cpp").unwrap(),
      vec![],
      proto::File::Cached(exec_id.into()),
      proto::File::Memory("hello\n".into()),
      proto::File::Memory("9 9   8\n2\n  4 4\t3 5\n3".into()),
      proto::File::Memory("9 9 8 2 4 4 3 5 3\n".into()),
      HashMap::new(),
    )
    .await
    .unwrap();

  assert_eq!(res.status, checker::Status::Accepted);
}
