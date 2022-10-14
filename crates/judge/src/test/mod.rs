mod checker;
mod compile;
mod generator;
mod sandbox;
mod validator;

#[cfg(test)]
fn init() {
  let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
    .is_test(true)
    .try_init();
}
