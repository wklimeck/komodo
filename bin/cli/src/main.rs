#[macro_use]
extern crate tracing;

use colored::Colorize;
use komodo_client::entities::config::cli;

mod command;
mod config;

async fn app() -> anyhow::Result<()> {
  dotenvy::dotenv().ok();
  logger::init(&config::cli_config().logging)?;

  info!(
    "Komodo CLI version: {}",
    env!("CARGO_PKG_VERSION").blue().bold()
  );

  match &config::cli_args().command {
    cli::Command::Execute { execution, .. } => {
      command::execute(execution.clone()).await
    }
    cli::Command::Database {
      command: cli::DatabaseCommand::Backup { .. },
    } => command::database::backup().await,
    cli::Command::Database {
      command: cli::DatabaseCommand::Restore { .. },
    } => command::database::restore().await,
    cli::Command::Database {
      command: cli::DatabaseCommand::Copy { .. },
    } => command::database::copy().await,
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
