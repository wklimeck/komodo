use std::sync::OnceLock;

use anyhow::{Context, anyhow};
use command::run_komodo_command;
use formatting::format_serror;
use komodo_client::api::execute::{
  BackupCoreDatabase, ClearRepoCache,
};
use reqwest::StatusCode;
use resolver_api::Resolve;
use serror::AddStatusCodeError;
use tokio::sync::Mutex;

use crate::{
  api::execute::ExecuteArgs, config::core_config,
  helpers::update::update_update,
};

/// Makes sure the method can only be called once at a time
fn clear_repo_cache_lock() -> &'static Mutex<()> {
  static CLEAR_REPO_CACHE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
  CLEAR_REPO_CACHE_LOCK.get_or_init(Default::default)
}

impl Resolve<ExecuteArgs> for ClearRepoCache {
  #[instrument(
    name = "ClearRepoCache",
    skip(user, update),
    fields(user_id = user.id, update_id = update.id)
  )]
  async fn resolve(
    self,
    ExecuteArgs { user, update }: &ExecuteArgs,
  ) -> Result<Self::Response, Self::Error> {
    if !user.admin {
      return Err(
        anyhow!("This method is admin only.")
          .status_code(StatusCode::UNAUTHORIZED),
      );
    }

    let _lock = clear_repo_cache_lock()
      .try_lock()
      .context("Clear already in progress...")?;

    let mut update = update.clone();

    let mut contents =
      tokio::fs::read_dir(&core_config().repo_directory)
        .await
        .context("Failed to read repo cache directory")?;

    loop {
      let path = match contents
        .next_entry()
        .await
        .context("Failed to read contents at path")
      {
        Ok(Some(contents)) => contents.path(),
        Ok(None) => break,
        Err(e) => {
          update.push_error_log(
            "Read Directory",
            format_serror(&e.into()),
          );
          continue;
        }
      };
      if path.is_dir() {
        match tokio::fs::remove_dir_all(&path)
          .await
          .context("Failed to clear contents at path")
        {
          Ok(_) => {}
          Err(e) => {
            update.push_error_log(
              "Clear Directory",
              format_serror(&e.into()),
            );
          }
        };
      }
    }

    update.finalize();
    update_update(update.clone()).await?;

    Ok(update)
  }
}

//

/// Makes sure the method can only be called once at a time
fn backup_database_lock() -> &'static Mutex<()> {
  static BACKUP_DATABASE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
  BACKUP_DATABASE_LOCK.get_or_init(Default::default)
}

impl Resolve<ExecuteArgs> for BackupCoreDatabase {
  #[instrument(
    name = "BackupCoreDatabase",
    skip(user, update),
    fields(user_id = user.id, update_id = update.id)
  )]
  async fn resolve(
    self,
    ExecuteArgs { user, update }: &ExecuteArgs,
  ) -> Result<Self::Response, Self::Error> {
    if !user.admin {
      return Err(
        anyhow!("This method is admin only.")
          .status_code(StatusCode::UNAUTHORIZED),
      );
    }

    let _lock = backup_database_lock()
      .try_lock()
      .context("Backup already in progress...")?;

    let mut update = update.clone();

    update_update(update.clone()).await?;

    let res = run_komodo_command(
      "Backup Core Database",
      None,
      "km database backup --yes",
    )
    .await;

    update.logs.push(res);
    update.finalize();

    update_update(update.clone()).await?;

    Ok(update)
  }
}
