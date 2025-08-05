use std::io::Read;

use anyhow::{Context, anyhow};
use colored::Colorize;
use komodo_client::KomodoClient;

use crate::config::{cli_args, cli_config};

pub mod database;

mod execute;

pub use execute::execute;

async fn komodo_client() -> anyhow::Result<KomodoClient> {
  let config = cli_config();
  let (Some(key), Some(secret)) =
    (&config.cli_key, &config.cli_secret)
  else {
    return Err(anyhow!("Must provide both cli_key and cli_secret"));
  };
  KomodoClient::new(&config.host, key, secret)
    .with_healthcheck()
    .await
}

fn wait_for_enter(press_enter_to: &str) -> anyhow::Result<()> {
  if cli_args().yes {
    println!("");
    return Ok(());
  }
  println!(
    "\nPress {} to {}\n",
    "ENTER".green(),
    press_enter_to.bold()
  );
  let buffer = &mut [0u8];
  std::io::stdin()
    .read_exact(buffer)
    .context("failed to read ENTER")?;
  Ok(())
}
