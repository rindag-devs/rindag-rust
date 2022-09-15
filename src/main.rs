pub mod hander;
pub mod service;

#[cfg(test)]
mod test;

use std::error::Error;

use clap::Parser;

use crate::service::{
  args,
  etc::{self, CONFIG},
};

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
