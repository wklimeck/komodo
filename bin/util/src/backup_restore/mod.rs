use std::{path::PathBuf, str::FromStr};

use anyhow::Context;
use environment_file::maybe_read_item_from_file;
use mungos::{init::MongoBuilder, mongodb::Database};
use serde::Deserialize;

pub mod backup;
pub mod restore;

#[derive(Deserialize)]
struct Env {
  /// The root folder to store timestamped backup folders in.
  #[serde(default = "default_backup_folder")]
  komodo_backup_folder: PathBuf,

  komodo_database_uri: Option<String>,
  komodo_database_uri_file: Option<PathBuf>,

  komodo_database_address: Option<String>,

  komodo_database_username: Option<String>,
  komodo_database_username_file: Option<PathBuf>,

  komodo_database_password: Option<String>,
  komodo_database_password_file: Option<PathBuf>,

  #[serde(default = "default_app_name")]
  komodo_database_app_name: String,

  #[serde(default = "default_db_name")]
  komodo_database_db_name: String,
}

fn default_backup_folder() -> PathBuf {
  // SAFE: /backup is a valid path.
  PathBuf::from_str("/backup").unwrap()
}

fn default_app_name() -> String {
  String::from("komodo-backup")
}

fn default_db_name() -> String {
  String::from("komodo")
}

async fn database(env: &Env) -> anyhow::Result<Database> {
  let mut db_builder = MongoBuilder::default();
  if let Some(uri) = maybe_read_item_from_file(
    env.komodo_database_uri_file.clone(),
    env.komodo_database_uri.clone(),
  ) {
    db_builder = db_builder.uri(uri);
  }
  if let Some(address) = &env.komodo_database_address {
    db_builder = db_builder.address(address);
  }
  if let Some(username) = maybe_read_item_from_file(
    env.komodo_database_username_file.clone(),
    env.komodo_database_username.clone(),
  ) {
    db_builder = db_builder.username(username);
  }
  if let Some(password) = maybe_read_item_from_file(
    env.komodo_database_password_file.clone(),
    env.komodo_database_password.clone(),
  ) {
    db_builder = db_builder.password(password);
  }
  let db = db_builder
    .app_name(&env.komodo_database_app_name)
    .build()
    .await
    .context("Failed to initialize database")?
    .database(&env.komodo_database_db_name);
  Ok(db)
}
