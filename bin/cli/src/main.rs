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

  match config::cli_args().command.clone() {
    cli::Command::Execute { execution } => {
      command::execute(execution).await
    }
    cli::Command::Database {
      command: cli::DatabaseCommand::Backup,
    } => {
      todo!()
    }
    cli::Command::Database {
      command: cli::DatabaseCommand::Restore { time },
    } => {
      todo!()
    }
    cli::Command::Database {
      command: cli::DatabaseCommand::Copy { target_uri },
    } => {
      todo!()
    }
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
