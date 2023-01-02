use std::time;

mod checker;
mod generator;
mod problem;
mod program;
mod sandbox;
mod validator;

pub fn async_test<F: std::future::Future>(f: F) -> F::Output {
  lazy_static! {
    static ref RT: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .expect("should create a tokio runtime");
  }
  let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
    .is_test(true)
    .try_init();

  RT.block_on(async {
    let res = f.await;
    // Delays waiting for the FileHandle to be freed.
    tokio::time::sleep(time::Duration::from_millis(100)).await;
    res
  })
}
