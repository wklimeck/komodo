use colored::Colorize;
use komodo_client::entities::optional_string;

use crate::config::cli_config;

pub async fn backup(yes: bool) -> anyhow::Result<()> {
  let config = cli_config();

  println!(
    "\n🦎  {} Database {} Utility  🦎",
    "Komodo".bold(),
    "Backup".green().bold()
  );
  println!(
    "\n{}\n",
    " - Backup all database contents to gzip compressed files."
      .dimmed()
  );
  if let Some(uri) = optional_string(&config.database.uri) {
    println!(
      "{}: {}",
      " - Source URI".dimmed(),
      sanitize_uri(&uri)
    );
  }
  if let Some(address) = optional_string(&config.database.address) {
    println!("{}: {address}", " - Source Address".dimmed());
  }
  if let Some(username) = optional_string(&config.database.username) {
    println!(
      "{}: {username}",
      " - Source Username".dimmed()
    );
  }
  println!(
    "{}: {}\n",
    " - Source Db Name".dimmed(),
    config.database.db_name,
  );
  println!(
    "{}: {:?}",
    " - Backups Folder".dimmed(),
    config.backups_folder
  );

  crate::command::wait_for_enter("start backup", yes)?;

  let db = database::init(&config.database).await?;

  database::utils::backup(&db, &config.backups_folder).await
}

pub async fn restore(yes: bool) -> anyhow::Result<()> {
  let config = cli_config();

  println!(
    "\n🦎  {} Database {} Utility  🦎",
    "Komodo".bold(),
    "Restore".purple().bold()
  );
  println!(
    "\n{}\n",
    " - Restores database contents from gzip compressed files."
      .dimmed()
  );
  if let Some(uri) = optional_string(&config.database_target.uri) {
    println!(
      "{}: {}",
      " - Target URI".dimmed(),
      sanitize_uri(&uri)
    );
  }
  if let Some(address) =
    optional_string(&config.database_target.address)
  {
    println!("{}: {address}", " - Target Address".dimmed());
  }
  if let Some(username) =
    optional_string(&config.database_target.username)
  {
    println!(
      "{}: {username}",
      " - Target Username".dimmed()
    );
  }
  println!(
    "{}: {}\n",
    " - Target Db Name".dimmed(),
    config.database_target.db_name,
  );
  println!(
    "{}: {:?}",
    " - Backups Folder".dimmed(),
    config.backups_folder
  );
  if let Some(restore_folder) = &config.restore_folder {
    println!("{}: {restore_folder:?}", " - Restore Folder".dimmed());
  }

  crate::command::wait_for_enter("start restore", yes)?;

  // Initialize the whole client to ensure the target database is indexed.
  let db = database::Client::new(&config.database_target).await?;

  database::utils::restore(
    &db.db,
    &config.backups_folder,
    config.restore_folder.as_deref(),
  )
  .await
}

pub async fn copy(yes: bool) -> anyhow::Result<()> {
  let config = cli_config();

  println!(
    "\n🦎  {} Database {} Utility  🦎",
    "Komodo".bold(),
    "Copy".blue().bold()
  );
  println!(
    "\n{}\n",
    " - Copies database contents to another database.".dimmed()
  );

  if let Some(uri) = optional_string(&config.database.uri) {
    println!(
      "{}: {}",
      " - Source URI".dimmed(),
      sanitize_uri(&uri)
    );
  }
  if let Some(address) = optional_string(&config.database.address) {
    println!("{}: {address}", " - Source Address".dimmed());
  }
  if let Some(username) = optional_string(&config.database.username) {
    println!(
      "{}: {username}",
      " - Source Username".dimmed()
    );
  }
  println!(
    "{}: {}\n",
    " - Source Db Name".dimmed(),
    config.database.db_name,
  );

  if let Some(uri) = optional_string(&config.database_target.uri) {
    println!(
      "{}: {}",
      " - Target URI".dimmed(),
      sanitize_uri(&uri)
    );
  }
  if let Some(address) =
    optional_string(&config.database_target.address)
  {
    println!("{}: {address}", " - Target Address".dimmed());
  }
  if let Some(username) =
    optional_string(&config.database_target.username)
  {
    println!(
      "{}: {username}",
      " - Target Username".dimmed()
    );
  }
  println!(
    "{}: {}",
    " - Target Db Name".dimmed(),
    config.database_target.db_name,
  );

  crate::command::wait_for_enter("start copy", yes)?;

  let source_db = database::init(&config.database).await?;
  // Initialize the full client to perform indexing
  let target_db =
    database::Client::new(&config.database_target).await?;

  database::utils::copy(&source_db, &target_db.db).await
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
