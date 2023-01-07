mod answer;
mod input;

use std::{collections::HashMap, time};

use futures::channel::mpsc;
use futures::{stream, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{checker, data, program, record, sandbox};

pub use self::answer::Answer;
pub use self::input::Input;

/// Parsed problem.
pub struct Problem {
  /// Subtasks of the problem.
  ///
  /// Each subtask will be scored independently,
  /// and the score of a single subtask is the maximum of the scores of the test data it contains.
  pub subtasks: Vec<Subtask>,

  /// Problem type.
  pub kind: Kind,

  /// Checker of the problem.
  /// If problem type is Interactive, it will be used as an interactor.
  pub checker: program::Source,

  /// Main correct solution of the problem.
  ///
  /// Used to generate answer files.
  /// And use this solution's results to check this problem and judge other solutions.
  pub standard_solution: program::Source,

  /// Extra files when compiling or running checker.
  pub user_copy_in: HashMap<String, data::Provider>,

  /// Extra files when running solution.
  pub judge_copy_in: HashMap<String, data::Provider>,
}

/// Type of the problem.
pub enum Kind {
  /// Batch problem (a.k.a. traditional problem).
  Batch,
  /// Interactive problem.
  Interactive,
  /// Submit answer problem.
  SubmitAnswer,
}

/// Test set of a subtask or test case.
#[derive(Debug, PartialEq, Eq, strum::EnumString, strum::Display, strum::EnumIter, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
pub enum Testset {
  Sample,
  Pretests,
  Main,
  Hack,
}

pub struct Subtask {
  pub id: usize,
  pub score: f32,
  pub dependences: Vec<usize>,
  pub testset: Testset,
  pub tests: Vec<Test>,
  pub time_limit: time::Duration,
  pub memory_limit: u64,
}

/// Parsed test (a pair of input file and output file).
pub struct Test {
  pub input: Input,
  pub answer: Answer,
}

impl Test {
  /// Run a solution on a single test and return the record.
  async fn judge(
    &self,
    testset: &Testset,
    subtask_id: usize,
    solution: &program::Executable,
    standard_solution: &program::Executable,
    checker: &checker::Checker,
    time_limit: time::Duration,
    memory_limit: u64,
    user_copy_in: &HashMap<String, sandbox::FileHandle>,
    judge_copy_in: &HashMap<String, sandbox::FileHandle>,
  ) -> record::Record {
    // Generate input file.
    let input_file = match self.input.make(user_copy_in.clone()).await {
      Ok(x) => x,
      Err(err) => {
        return record::Record::new_system_error(
          &("input file generated failed: ".to_string() + &err.to_string()),
        );
      }
    };

    // Runs the given solution while executing the standard solution to generate answer data.
    let (answer_file, execute_result) = futures::join!(
      self.answer.make(
        &standard_solution,
        input_file.clone(),
        judge_copy_in.clone(),
        time_limit,
        memory_limit
      ),
      solution.judge_batch(
        vec![].clone(),
        input_file.clone(),
        judge_copy_in.clone(),
        time_limit,
        memory_limit
      ),
    );

    let answer_file = match answer_file {
      Ok(f) => f,
      Err(err) => {
        return record::Record::new_system_error(
          &("answer file generated failed: ".to_string() + &err.to_string()),
        );
      }
    };

    // Handle the situation where the solution program exits abnormally.
    if execute_result.0.status != sandbox::Status::Accepted {
      return record::Record::new_interrupted(&execute_result.0);
    }

    let output_file = execute_result.1.unwrap();
    let sol_result = execute_result.0;

    // Run the checker to see if the output is correct.
    let checker_result = checker
      .check(
        vec![
          "--testset".to_string(),
          testset.to_string(),
          "--group".to_string(),
          subtask_id.to_string(),
        ],
        input_file,
        output_file,
        answer_file,
        user_copy_in.clone(),
      )
      .await;

    match checker_result {
      Ok(checker_output) => record::Record::new_checked(&sol_result, &checker_output),
      Err(err) => record::Record::new_system_error(
        &("checker execute failed: ".to_string() + &err.to_string()),
      ),
    }
  }
}

impl Subtask {
  /// Run a solution on a subtask and return the score of subtask and each test's record.
  ///
  /// The score is unscaled (in range \[0,1\]),
  /// which means it will ignore the `score` felid of `self`．
  pub async fn judge(
    &self,
    solution: &program::Executable,
    standard_solution: &program::Executable,
    checker: &checker::Checker,
    user_copy_in: &HashMap<String, sandbox::FileHandle>,
    judge_copy_in: &HashMap<String, sandbox::FileHandle>,
    status_tx: Option<mpsc::UnboundedSender<Response>>,
  ) -> (f32, Vec<record::Record>) {
    let records: Vec<_> =
      stream::FuturesOrdered::from_iter(self.tests.iter().enumerate().map(|t| {
        t.1.judge(
          &self.testset,
          self.id,
          &solution,
          &standard_solution,
          &checker,
          self.time_limit,
          self.memory_limit,
          &user_copy_in,
          &judge_copy_in,
        )
      }))
      .then(|f| async {
        if let Some(mut tx) = status_tx.clone() {
          _ = tx.send(Response::CompleteOne { record: f.clone() });
        }
        f
      })
      .collect()
      .await;

    let score = records.iter().fold(1f32, |a, b| a.min(b.score));

    if let Some(mut tx) = status_tx.clone() {
      _ = tx.send(Response::Finished {
        score,
        records: records.clone(),
      });
    }

    return (score, records);
  }
}

/// Judgement status of an entire problem.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Response {
  /// A single test case judge finished.
  CompleteOne { record: record::Record },
  /// The subject assessment is completed.
  Finished {
    score: f32,
    records: Vec<record::Record>,
  },
}
