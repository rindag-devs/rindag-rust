pub mod args;
pub mod etc;
pub mod result;
pub mod sandbox;
pub mod task;

#[cfg(test)]
mod test;
pub mod testlib;

use std::error::Error;

use clap::Parser;

use crate::etc::CONFIG;
pub use crate::result::Status;

extern crate pretty_env_logger;
#[macro_use]
extern crate lazy_static;
extern crate log;

fn main() -> Result<(), Box<dyn Error>> {
  let args = args::Args::parse();
  etc::load_config(&args.config_search_path);
  dbg!(&*CONFIG);
  return Ok(());
}
