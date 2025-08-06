use std::io::Read;

use anyhow::{Context, anyhow};
use colored::Colorize;
use komodo_client::KomodoClient;
use tokio::sync::OnceCell;

use crate::config::cli_config;

pub mod database;

mod execute;

pub use execute::execute;

async fn komodo_client() -> anyhow::Result<&'static KomodoClient> {
  static KOMODO_CLIENT: OnceCell<KomodoClient> =
    OnceCell::const_new();
  KOMODO_CLIENT
    .get_or_try_init(|| async {
      let config = cli_config();
      let (Some(key), Some(secret)) =
        (&config.cli_key, &config.cli_secret)
      else {
        return Err(anyhow!(
          "Must provide both cli_key and cli_secret"
        ));
      };
      KomodoClient::new(&config.host, key, secret)
        .with_healthcheck()
        .await
    })
    .await
}

fn wait_for_enter(
  press_enter_to: &str,
  skip: bool,
) -> anyhow::Result<()> {
  if skip {
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
