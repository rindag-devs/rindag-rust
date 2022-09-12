pub mod hander;
pub mod service;

use crate::service::{
  args,
  etc::{self, CONFIG},
};
use clap::Parser;
use std::error::Error;

#[macro_use]
extern crate lazy_static;

fn main() -> Result<(), Box<dyn Error>> {
  let args = args::Args::parse();
  etc::load_config(&args.config_search_path);
  dbg!(&*CONFIG);
  return Ok(());
}
