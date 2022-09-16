use crate::{testlib, Status};

#[test]
fn test_parse_output() {
  assert_eq!(
    testlib::parse_output("ok you win\n3 steps.", false),
    (Status::Accepted, 1.0f32, "ac you win\n3 steps.".to_string())
  );

  assert_eq!(
    testlib::parse_output("wrong answer you lose\n12 steps.", false),
    (
      Status::WrongAnswer,
      0.0f32,
      "wa you lose\n12 steps.".to_string()
    )
  );

  assert_eq!(
    testlib::parse_output("points 0.12 you used 12 / 100 moves", false),
    (
      Status::PartiallyCorrect,
      0.12f32,
      "pc you used 12 / 100 moves".to_string()
    )
  );

  assert_eq!(
    testlib::parse_output("wrong output format \t \textra spaces\n\t\t", false),
    (
      Status::PresentationError,
      0.0f32,
      "pe extra spaces".to_string()
    )
  );

  assert_eq!(
    testlib::parse_output("status(time_limit_exceeded)\nscore(1)", false),
    (
      Status::TimeLimitExceeded,
      1.0f32,
      "status(time_limit_exceeded)\nscore(1)".to_string()
    )
  );
}
