use clap::Parser;
use shadow_rs::shadow;

shadow!(build);

// Command line args
#[derive(Parser)]
#[clap(version = build::CLAP_LONG_VERSION)]
#[clap(about = clap::crate_description!(), long_about = None)]
pub struct Args {
  #[clap(short, long, value_parser)]
  pub config_search_path: Vec<String>,
}
