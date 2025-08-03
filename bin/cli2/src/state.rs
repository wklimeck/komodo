use std::sync::OnceLock;

use clap::Parser;

use crate::config::CliArgs;

pub fn cli_args() -> &'static CliArgs {
  static CLI_ARGS: OnceLock<CliArgs> = OnceLock::new();
  CLI_ARGS.get_or_init(CliArgs::parse)
}
