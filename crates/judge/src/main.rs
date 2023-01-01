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

use std::collections::HashMap;

use actix_web::{get, middleware::Logger, web, Responder};

pub use crate::{args::ARGS, etc::CONFIG};

#[macro_use]
extern crate lazy_static;
extern crate log;

#[get("/")]
async fn greet(query: web::Query<HashMap<String, String>>) -> impl Responder {
  match query.get("name") {
    Some(name) => format!("hello name: {}", name),
    None => "hello default".to_string(),
  }
}

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
  env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
    .is_test(false)
    .try_init()
    .unwrap();

  log::info!("server start");

  actix_web::HttpServer::new(|| actix_web::App::new().wrap(Logger::default()).service(greet))
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
