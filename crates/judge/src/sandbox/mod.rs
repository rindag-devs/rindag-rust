mod client;
mod file;
mod request;
mod response;

mod proto {
  tonic::include_proto!("pb");
}

pub use {
  file::FileHandle,
  request::{Cmd, Request},
  response::{ExecuteResult, ResponseResult, Status},
};
