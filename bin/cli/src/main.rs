#[macro_use]
extern crate tracing;

use colored::Colorize;
use komodo_client::entities::config::cli;

mod command;
mod config;

async fn app() -> anyhow::Result<()> {
  dotenvy::dotenv().ok();
  logger::init(&config::cli_config().cli_logging)?;

  info!(
    "Komodo CLI version: {}",
    env!("CARGO_PKG_VERSION").blue().bold()
  );

  match &config::cli_args().command {
    cli::Command::Execute { execution, yes, .. } => {
      command::execute(execution.clone(), *yes).await
    }
    cli::Command::Database {
      command: cli::DatabaseCommand::Backup { yes, .. },
    } => command::database::backup(*yes).await,
    cli::Command::Database {
      command: cli::DatabaseCommand::Restore { yes, .. },
    } => command::database::restore(*yes).await,
    cli::Command::Database {
      command: cli::DatabaseCommand::Copy { yes, .. },
    } => command::database::copy(*yes).await,
  }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let mut term_signal = tokio::signal::unix::signal(
    tokio::signal::unix::SignalKind::terminate(),
  )?;
  tokio::select! {
    res = tokio::spawn(app()) => res?,
    _ = term_signal.recv() => Ok(()),
  }
}
