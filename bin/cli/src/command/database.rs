use colored::Colorize;

use crate::config::cli_config;

pub async fn backup() -> anyhow::Result<()> {
  let config = cli_config();

  println!("");
  println!(
    "🦎 {} Database {} Utility 🦎",
    "Komodo".bold(),
    "Backup".green().bold()
  );
  println!(
    "{}",
    "Backup all database contents to gzip compressed files.".dimmed()
  );
  println!("");
  println!("{}: {:?}", "Root Folder".dimmed(), config.backup_folder);

  crate::command::wait_for_enter("start backup")?;

  let db = database::init(&config.database).await?;

  database::utils::backup(&db, &config.backup_folder).await
}

pub async fn restore() -> anyhow::Result<()> {
  let config = cli_config();

  println!("");
  println!(
    "🦎 {} Database {} Utility 🦎",
    "Komodo".bold(),
    "Restore".red().bold()
  );
  println!(
    "{}",
    "Restores database contents from gzip compressed files.".dimmed()
  );
  println!("");
  println!("{}: {:?}", "Root Folder".dimmed(), config.backup_folder);
  if let Some(restore_folder) = &config.restore_folder {
    println!("{restore_folder:?}: {:?}", "Restore Folder".dimmed());
  }

  crate::command::wait_for_enter("start restore")?;

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

  println!("");
  println!(
    "🦎 {} Database {} Utility 🦎",
    "Komodo".bold(),
    "Copy".blue().bold()
  );
  println!(
    "{}",
    "Copies database contents to another database.".dimmed()
  );

  crate::command::wait_for_enter("start copy")?;

  let source_db = database::init(&config.database).await?;
  // Initialize the full client to perform indexing
  let target_db =
    database::Client::new(&config.database_copy).await?;

  database::utils::copy(&source_db, &target_db.db).await
}
