#[macro_use]
extern crate tracing;

use serde::Deserialize;

mod backup_restore;
mod copy_database;

#[derive(Deserialize, Debug)]
enum Mode {
  BackupDatabase,
  RestoreDatabase,
  CopyDatabase,
}

#[derive(Deserialize)]
struct Env {
  mode: Mode,
}

async fn app() -> anyhow::Result<()> {
  dotenvy::dotenv().ok();
  tracing_subscriber::fmt::init();

  let env = envy::from_env::<Env>()?;

  info!("Komodo Util version: v{}", env!("CARGO_PKG_VERSION"));
  info!("Mode: {:?}", env.mode);

  match env.mode {
    Mode::CopyDatabase => copy_database::main().await,
    Mode::BackupDatabase => backup_restore::backup::main().await,
    Mode::RestoreDatabase => backup_restore::restore::main().await,
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
