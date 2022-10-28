#[cfg(test)]
mod test;

pub mod args;
pub mod builtin;
pub mod checker;
pub mod compile;
pub mod etc;
pub mod file;
pub mod generator;
pub mod judge;
pub mod problem;
pub mod result;
pub mod sandbox;
pub mod validator;
pub mod workflow;

use std::error::Error;

pub use crate::{args::ARGS, etc::CONFIG};

#[macro_use]
extern crate lazy_static;
extern crate log;

fn main() -> Result<(), Box<dyn Error>> {
  todo!();
}
