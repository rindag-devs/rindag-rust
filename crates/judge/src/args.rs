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

  command: Option<String>,

  #[clap(long)]
  exact: bool,

  #[clap(long)]
  nocapture: bool,
}

lazy_static! {
  pub static ref ARGS: Args = Args::parse();
}
