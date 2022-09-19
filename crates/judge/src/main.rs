pub mod args;
pub mod etc;
pub mod result;
pub mod sandbox;
pub mod task;

#[cfg(test)]
mod test;
pub mod testlib;

use std::error::Error;

pub use crate::{args::ARGS, etc::CONFIG, result::Status, sandbox::client::CLIENT};

#[macro_use]
extern crate lazy_static;
extern crate log;

fn main() -> Result<(), Box<dyn Error>> {
  dbg!(&*CONFIG);
  return Ok(());
}
