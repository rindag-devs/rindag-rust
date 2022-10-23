use std::{collections::HashMap, mem, pin::Pin, str::FromStr, sync::Arc, time};

use futures::{stream, Future, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, watch};

use crate::{
  etc, file, result,
  sandbox::{self, proto},
};

/// Parsed problem.
pub struct Problem {
  /// Subtasks of the problem.
  /// Each subtask will be scored independently,
  /// and the score of a single subtask is the maximum of the scores of the test data it contains.
  pub subtasks: Vec<Subtask>,

  /// Problem type.
  pub kind: Kind,

  /// Checker of the problem.
  /// If problem type is Interactive, it will be used as an interactor.
  pub checker: SourceCode,

  /// Extra files when compiling or running checker.
  pub user_copy_in: HashMap<String, file::File>,

  /// Extra files when running solution.
  pub judge_copy_in: HashMap<String, file::File>,
}

/// Type of the problem.
pub enum Kind {
  Batch,
  Interactive,
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
  pub score: f32,
  pub dependences: Vec<usize>,
  pub testset: Testset,
  pub tests: Vec<Test>,
  pub time_limit: time::Duration,
  pub memory_limit: u64,
}

// Parsed test (a pair of input file and output file).
pub struct Test {
  pub input: file::File,
  pub answer: file::File,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceCode {
  pub lang: String,
  pub data: file::File,
}

impl Subtask {
  /// Run a solution on a subtask and returns the score of subtask and each test's record.
  ///
  /// The score is unscaled (in range \[0,1\]),
  /// which means it will ignore the `score` felid of `self`．
  async fn judge(
    &self,
    sandbox: Arc<sandbox::Client>,
    subtask_id: u32,
    exec: String,
    exec_lang: &etc::LangCfg,
    checker: String,
    checker_lang: &etc::LangCfg,
    user_copy_in: HashMap<String, proto::File>,
    judge_copy_in: HashMap<String, proto::File>,
  ) -> (f32, Vec<result::Record>) {
    let mut inf = Vec::with_capacity(self.tests.len());
    let mut ans = Vec::with_capacity(self.tests.len());

    // Upload the test files to sandbox.
    for test in &self.tests {
      for (f, file_id) in [(&test.input, &mut inf), (&test.answer, &mut ans)] {
        let content = f.get_content();
        file_id.push(match sandbox.file_add(content).await {
          Ok(x) => x,
          Err(err) => {
            return (
              0.,
              vec![result::Record::new(err.into(), None); self.tests.len()],
            )
          }
        });
      }
    }

    let mut coroutines: Vec<Pin<Box<dyn Future<Output = ()>>>> =
      Vec::with_capacity(self.tests.len() * 2);
    let mut records = Vec::with_capacity(self.tests.len());
    for i in 0..self.tests.len() {
      let (res_tx, res_rx) = oneshot::channel();
      let (ouf_tx, ouf_rx) = oneshot::channel();
      {
        let inf = inf[i].clone();
        let exec = exec.clone();
        let judge_copy_in = judge_copy_in.clone();
        let sandbox = sandbox.clone();

        coroutines.push(Box::pin(async move {
          let (result, ouf) = sandbox
            .judge_batch(
              &exec_lang,
              vec![],
              proto::File::Cached(exec.into()),
              proto::File::Cached(inf.into()),
              judge_copy_in,
              self.time_limit,
              self.memory_limit,
            )
            .await;
          res_tx.send(result).unwrap();
          ouf_tx.send(ouf).unwrap();
        }));
      }

      {
        let inf = inf[i].clone();
        let sandbox = sandbox.clone();
        let checker = checker.clone();
        let user_copy_in = user_copy_in.clone();
        let ans = ans[i].clone();
        let testset = self.testset.clone();
        let (record_tx, record_rx) = oneshot::channel();
        records.push(record_rx);

        coroutines.push(Box::pin(async move {
          let ouf = ouf_rx.await.unwrap();
          let res = res_rx.await.unwrap();
          if ouf.is_none() {
            record_tx.send(result::Record::new(res, None)).unwrap();
            return;
          }
          let ouf = ouf.unwrap();
          let checker_res = sandbox
            .check(
              &checker_lang,
              vec![
                "--testset".to_string(),
                testset.to_string(),
                "--group".to_string(),
                subtask_id.to_string(),
              ],
              proto::File::Cached(checker.into()),
              proto::File::Cached(inf.into()),
              proto::File::Cached(ouf.into()),
              proto::File::Cached(ans.into()),
              user_copy_in,
            )
            .await;

          match checker_res {
            Ok(c) => record_tx.send(result::Record::new(res, Some(c))).unwrap(),
            Err(err) => {
              let mut record = result::Record::new(res, None);
              record.checker_message = format!("error: {}", err.to_string());
              record_tx.send(record).unwrap()
            }
          };
        }));
      }
    }
    futures::future::join_all(coroutines).await;

    let records = futures::future::join_all(
      records
        .into_iter()
        .map(|r| async { r.await.unwrap() })
        .collect::<Vec<_>>(),
    )
    .await;

    let score = records.iter().fold(f32::INFINITY, |a, b| a.min(b.score));

    return (score, records);
  }
}

impl Problem {
  pub async fn judge(
    &self,
    sandbox: Arc<sandbox::Client>,
    sol_code: SourceCode,
  ) -> result::JudgeResult {
    // Prepare copy in files.
    let user_copy_in: HashMap<String, proto::File> = match stream::iter(&self.user_copy_in)
      .then(|f| async {
        Ok((
          f.0.to_string(),
          proto::File::Cached(sandbox.file_add(f.1.get_content()).await?.into()),
        ))
      })
      .collect::<Vec<_>>()
      .await
      .into_iter()
      .collect::<Result<_, tonic::Status>>()
    {
      Ok(x) => x,
      Err(err) => return err.into(),
    };
    let judge_copy_in: HashMap<String, proto::File> = match stream::iter(&self.judge_copy_in)
      .then(|f| async {
        Ok((
          f.0.to_string(),
          proto::File::Cached(sandbox.file_add(f.1.get_content()).await?.into()),
        ))
      })
      .collect::<Vec<_>>()
      .await
      .into_iter()
      .collect::<Result<_, tonic::Status>>()
    {
      Ok(x) => x,
      Err(err) => return err.into(),
    };

    // Compile solution code.
    let sol_code_id = match sandbox.file_add(sol_code.data.get_content()).await {
      Ok(x) => x,
      Err(err) => return err.into(),
    };
    let sol_lang = match etc::LangCfg::from_str(&sol_code.lang) {
      Ok(l) => l,
      Err(err) => {
        return result::JudgeResult::CompileError {
          message: err.to_string(),
        }
      }
    };

    let sol_exec_id = match sandbox
      .compile(
        &sol_lang,
        vec![],
        proto::File::Cached(sol_code_id.into()),
        user_copy_in.clone(),
      )
      .await
    {
      Ok(x) => x,
      Err(err) => return result::JudgeResult::from_compile_error(err),
    };

    // Compile checker.
    let checker_code_id = match sandbox.file_add(self.checker.data.get_content()).await {
      Ok(x) => x,
      Err(err) => return err.into(),
    };
    let checker_lang = match etc::LangCfg::from_str(&self.checker.lang) {
      Ok(l) => l,
      Err(err) => {
        return result::JudgeResult::CompileError {
          message: err.to_string(),
        }
      }
    };

    let checker_exec_id = match sandbox
      .compile(
        &checker_lang,
        vec![],
        proto::File::Cached(checker_code_id.into()),
        user_copy_in.clone(),
      )
      .await
    {
      Ok(x) => x,
      Err(err) => return result::JudgeResult::from_compile_error(err),
    };

    let mut score_tx = Vec::with_capacity(self.subtasks.len());
    let mut score_rx = Vec::with_capacity(self.subtasks.len());
    let mut coroutines = futures::stream::FuturesOrdered::new();
    for _ in 0..self.subtasks.len() {
      let (tx, rx) = watch::channel(0.);
      score_tx.push(Some(tx));
      score_rx.push(rx);
    }
    for (i, subtask) in self.subtasks.iter().enumerate() {
      let score_tx = mem::replace(&mut score_tx[i], None).unwrap();
      let dep_score_rx: Vec<_> = subtask
        .dependences
        .iter()
        .map(|d| score_rx[*d].clone())
        .collect();
      let sandbox = sandbox.clone();
      let sol_exec_id = sol_exec_id.clone();
      let sol_lang = sol_lang.clone();
      let checker_exec_id = checker_exec_id.clone();
      let checker_lang = checker_lang.clone();
      let user_copy_in = user_copy_in.clone();
      let judge_copy_in = judge_copy_in.clone();
      coroutines.push_back(async move {
        let mut score = stream::iter(dep_score_rx)
          .fold(1f32, |score, mut rx| async move {
            score.min({
              rx.changed().await.unwrap();
              (*rx.borrow()).clone()
            })
          })
          .await;
        if score == 0. {
          score_tx.send(0.).unwrap();
          return vec![result::RECORD_SKIPPED.clone(); subtask.tests.len()];
        }
        let (cur_score, result) = subtask
          .judge(
            sandbox,
            i as u32,
            sol_exec_id,
            &sol_lang,
            checker_exec_id,
            &checker_lang,
            user_copy_in,
            judge_copy_in,
          )
          .await;
        score = score.min(cur_score);
        score_tx.send(score).unwrap();
        return result;
      });
    }

    let subtask_results: Vec<_> = coroutines.collect().await;

    let mut sum_score = 0.;
    for (i, subtask) in self.subtasks.iter().enumerate() {
      sum_score += subtask.score * *score_rx[i].borrow();
    }

    return result::JudgeResult::Ok {
      score: sum_score,
      results: subtask_results,
    };
  }
}
