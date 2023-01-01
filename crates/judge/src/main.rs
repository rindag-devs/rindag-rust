#[cfg(test)]
mod test;

pub mod args;
pub mod builtin;
pub mod checker;
pub mod etc;
pub mod file;
pub mod generator;
pub mod judge;
pub mod lang;
pub mod problem;
pub mod program;
pub mod result;
pub mod sandbox;
pub mod validator;

pub use crate::{args::ARGS, etc::CONFIG};

#[macro_use]
extern crate lazy_static;
extern crate log;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
  todo!()
}
