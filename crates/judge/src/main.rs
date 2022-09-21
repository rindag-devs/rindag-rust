#[cfg(test)]
mod test;

pub mod args;
pub mod checker;
pub mod compile;
pub mod etc;
pub mod generator;
pub mod judge;
pub mod result;
pub mod sandbox;
pub mod testlib;
pub mod validator;

use std::error::Error;

pub use crate::{args::ARGS, etc::CONFIG};

#[macro_use]
extern crate lazy_static;
extern crate log;

fn main() -> Result<(), Box<dyn Error>> {
  dbg!(&*CONFIG);
  return Ok(());
}
