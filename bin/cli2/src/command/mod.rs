use clap::Subcommand;
use komodo_client::api::execute::Execution;

// mod execute;

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
  /// Runs an execution
  Execute {
    #[command(subcommand)]
    execution: Execution,
  },
  // Room for more
}
