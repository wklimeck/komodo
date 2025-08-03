#[macro_use]
extern crate tracing;

use std::io::Read;

use anyhow::Context;
use colored::Colorize;

mod command;
mod config;
mod state;

async fn app() -> anyhow::Result<()> {
  dotenvy::dotenv().ok();
  Ok(())
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

fn wait_for_enter(press_enter_to: &str) -> anyhow::Result<()> {
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
