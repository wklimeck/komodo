use std::io::Read;

use anyhow::{Context, anyhow};
use colored::Colorize;
use komodo_client::KomodoClient;
use tokio::sync::OnceCell;

use crate::config::cli_config;

pub mod database;
pub mod execute;
pub mod update;

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
    println!();
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

/// Sanitizes uris of the form:
/// `protocol://username:password@address`
fn sanitize_uri(uri: &str) -> String {
  // protocol: `mongodb`
  // credentials_address: `username:password@address`
  let Some((protocol, credentials_address)) = uri.split_once("://")
  else {
    // If no protocol, return as-is
    return uri.to_string();
  };

  // credentials: `username:password`
  let Some((credentials, address)) =
    credentials_address.split_once('@')
  else {
    // If no credentials, return as-is
    return uri.to_string();
  };

  match credentials.split_once(':') {
    Some((username, _)) => {
      format!("{protocol}://{username}:*****@{address}")
    }
    None => {
      format!("{protocol}://*****@{address}")
    }
  }
}
