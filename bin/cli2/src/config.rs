use clap::Parser;

use crate::command::Command;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
  /// The command to run
  #[command(subcommand)]
  pub command: Command,
}
