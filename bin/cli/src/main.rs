#[macro_use]
extern crate tracing;

use colored::Colorize;
use komodo_client::entities::config::cli;

use crate::config::cli_config;

mod command;
mod config;

async fn app() -> anyhow::Result<()> {
  dotenvy::dotenv().ok();
  println!(
    "{}: Komodo CLI version: {}",
    "INFO".green(),
    env!("CARGO_PKG_VERSION").blue().bold()
  );
  logger::init(&config::cli_config().cli_logging)?;

  match &config::cli_args().command {
    cli::Command::Config { unsanitized } => {
      if *unsanitized {
        println!("\n{:#?}", cli_config());
      } else {
        println!("\n{:#?}", cli_config().sanitized());
      }
      Ok(())
    }
    cli::Command::Execute { execution, yes, .. } => {
      command::execute::handle(execution, *yes).await
    }
    cli::Command::Update { command } => {
      command::update::handle(command).await
    }
    cli::Command::Database { command } => {
      command::database::handle(command).await
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
