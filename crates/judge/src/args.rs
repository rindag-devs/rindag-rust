use clap::Parser;
use shadow_rs::shadow;

shadow!(build);

// Command line args
#[derive(Parser, Default)]
#[clap(version = build::CLAP_LONG_VERSION)]
#[clap(about = clap::crate_description!(), long_about = None)]
pub struct Args {
  #[clap(short, long, value_parser)]
  pub config_search_path: Vec<String>,
}

lazy_static! {
  pub static ref ARGS: Args = if cfg!(test) {
    Args::default()
  } else {
    Args::parse()
  };
}
