#[macro_use]
extern crate tracing;

use anyhow::Context;
use komodo_client::entities::config::cli;

use crate::config::cli_config;

mod command;
mod config;

async fn app() -> anyhow::Result<()> {
  dotenvy::dotenv().ok();
  logger::init(&config::cli_config().cli_logging)?;

  match &config::cli_args().command {
    cli::Command::Config {
      all_profiles,
      debug,
      unsanitized,
    } => {
      let mut config = if *unsanitized {
        cli_config().clone()
      } else {
        cli_config().sanitized()
      };
      if !*all_profiles {
        config.profile = Default::default();
      }
      if *debug {
        println!("\n{config:#?}");
      } else {
        println!(
          "\nCLI Config {}",
          serde_json::to_string_pretty(&config)
            .context("Failed to serialize config for pretty print")?
        );
      }
      Ok(())
    }
    cli::Command::List(list) => command::list::handle(list).await,
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
