pub mod args;
pub mod etc;
pub mod result;
pub mod sandbox;
pub mod task;
pub mod testlib;

#[cfg(test)]
mod test;

use std::error::Error;

pub use crate::{args::ARGS, etc::CONFIG};

#[macro_use]
extern crate lazy_static;
extern crate log;

fn main() -> Result<(), Box<dyn Error>> {
  dbg!(&*CONFIG);
  return Ok(());
}
