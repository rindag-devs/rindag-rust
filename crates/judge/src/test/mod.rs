mod checker;
mod compile;
mod generator;
mod problem;
mod sandbox;
mod validator;
mod workflow;

#[cfg(test)]
fn init() {
  let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
    .is_test(true)
    .try_init();
}

#[cfg(test)]
fn test_rt() -> &'static tokio::runtime::Runtime {
  lazy_static! {
    static ref RT: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .expect("should create a tokio runtime");
  }
  &RT
}
