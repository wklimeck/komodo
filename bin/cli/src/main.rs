#[macro_use]
extern crate tracing;

use anyhow::Context;
use komodo_client::entities::config::cli::args;

use crate::config::cli_config;

mod command;
mod config;

async fn app() -> anyhow::Result<()> {
  dotenvy::dotenv().ok();
  logger::init(&config::cli_config().cli_logging)?;

  match &config::cli_args().command {
    args::Command::Config {
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
    args::Command::List(list) => command::list::handle(list).await,
    args::Command::Container(container) => {
      command::container::handle(container).await
    }
    args::Command::Execute(args) => {
      command::execute::handle(&args.execution, args.yes).await
    }
    args::Command::Update { command } => {
      command::update::handle(command).await
    }
    args::Command::Database { command } => {
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
