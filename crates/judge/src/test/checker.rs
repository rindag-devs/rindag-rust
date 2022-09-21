use std::collections::HashMap;

use crate::{
  checker::{self, Output},
  result,
  sandbox::{self, proto},
  testlib, CONFIG,
};

#[test]
fn test_parse_output() {
  assert_eq!(
    Output::parse("ok you win\n3 steps."),
    Output {
      status: result::Status::Accepted,
      score: 1.0f32,
      message: "ac you win\n3 steps.".to_string()
    }
  );

  assert_eq!(
    Output::parse("wrong answer you lose\n12 steps."),
    Output {
      status: result::Status::WrongAnswer,
      score: 0.0f32,
      message: "wa you lose\n12 steps.".to_string()
    }
  );

  assert_eq!(
    Output::parse("points 0.12 you used 12 / 100 moves"),
    Output {
      status: result::Status::PartiallyCorrect,
      score: 0.12f32,
      message: "pc you used 12 / 100 moves".to_string()
    }
  );

  assert_eq!(
    Output::parse("wrong output format \t \textra spaces\n\t\t"),
    Output {
      status: result::Status::PresentationError,
      score: 0.0f32,
      message: "pe extra spaces".to_string()
    }
  );

  assert_eq!(
    Output::parse("status(time_limit_exceeded)\nscore(1)"),
    Output {
      status: result::Status::TimeLimitExceeded,
      score: 1.0f32,
      message: "status(time_limit_exceeded)\nscore(1)".to_string()
    }
  );
}

#[tokio::test]
async fn test_builtin_checker() {
  let sandbox = sandbox::Client::from_global_config().await;
  let checker = checker::Builtin::get_checker_as_file("ncmp").unwrap();

  let exec_id = sandbox
    .compile(
      &CONFIG.lang["cpp"],
      checker,
      [(
        "testlib.h".to_string(),
        proto::File::Memory(testlib::TESTLIB_SOURCE.into()),
      )]
      .into(),
    )
    .await
    .unwrap();

  let res = sandbox
    .run_checker(
      &CONFIG.lang["cpp"],
      vec![],
      proto::File::Cached(exec_id.into()),
      proto::File::Memory("hello\n".into()),
      proto::File::Memory("9 9   8\n2\n  4 4\t3 5\n3".into()),
      proto::File::Memory("9 9 8 2 4 4 3 5 3\n".into()),
      HashMap::new(),
    )
    .await
    .unwrap();

  assert_eq!(res.status, result::Status::Accepted);
}
