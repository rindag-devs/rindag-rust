use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::{
  builtin,
  checker::{self, Output},
  lang, program, sandbox,
};

#[test]
fn test_parse_output() {
  super::init();

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

#[test]
fn test_builtin_checker() {
  super::test_rt().block_on(async {
    super::init();

    let src = program::Source {
      lang: lang::Lang::from_str("cpp").unwrap(),
      data: builtin::File::from_str("checker:ncmp.cpp").unwrap().into(),
    };

    let chk = checker::Checker::from(
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

    let res = chk
      .check(
        vec![],
        Arc::new(sandbox::FileHandle::upload("hello\n".as_bytes()).await),
        Arc::new(sandbox::FileHandle::upload("9 9   8\n2\n  4 4\t3 5\n3".as_bytes()).await),
        Arc::new(sandbox::FileHandle::upload("9 9 8 2 4 4 3 5 3\n".as_bytes()).await),
        HashMap::new(),
      )
      .await
      .unwrap();

    assert_eq!(res.status, checker::Status::Accepted);
  });
}
