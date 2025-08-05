use colored::Colorize;

use crate::config::cli_config;

pub async fn backup() -> anyhow::Result<()> {
  let config = cli_config();

  println!(
    "🦎 {} database {} utility 🦎",
    "Komodo".bold(),
    "backup".green()
  );
  println!(
    "{}",
    "Backup all database contents to gzip compressed files.".dimmed()
  );
  println!("{}: {:?}", "Root Folder".dimmed(), config.backup_folder);

  crate::command::wait_for_enter("start backup")?;

  let db = database::init(&config.database).await?;

  database::utils::backup(&db, &config.backup_folder).await
}

pub async fn restore() -> anyhow::Result<()> {
  let config = cli_config();

  // Initialize the whole client to ensure the target database is indexed.
  let db = database::Client::new(&config.database).await?;

  database::utils::restore(
    &db.db,
    &config.backup_folder,
    config.restore_folder.as_deref(),
  )
  .await
}

pub async fn copy() -> anyhow::Result<()> {
  let config = cli_config();

  let source_db = database::init(&config.database).await?;
  // Initialize the full client to perform indexing
  let target_db =
    database::Client::new(&config.database_copy).await?;

  database::utils::copy(&source_db, &target_db.db).await
}
