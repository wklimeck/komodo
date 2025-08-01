use std::{path::PathBuf, str::FromStr};

use anyhow::Context;
use async_compression::tokio::bufread::GzipDecoder;
use environment_file::maybe_read_item_from_file;
use futures_util::{
  StreamExt, TryStreamExt, stream::FuturesUnordered,
};
use mungos::{
  init::MongoBuilder,
  mongodb::{bson::Document, options::InsertManyOptions},
};
use serde::Deserialize;
use tokio::io::BufReader;

#[derive(Deserialize)]
struct Env {
  /// The root folder to store timestamped backup folders in.
  #[serde(default = "default_backup_folder")]
  komodo_backup_folder: PathBuf,

  /// A specific dated folder to restore, relative to `KOMODO_BACKUP_FOLDER`.
  /// If not provided, will use the most recent folder.
  komodo_restore_folder: Option<PathBuf>,

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

fn default_app_name() -> String {
  String::from("komodo-restore")
}

fn default_db_name() -> String {
  String::from("komodo")
}

fn default_backup_folder() -> PathBuf {
  // SAFE: /backup is a valid path.
  PathBuf::from_str("/backup").unwrap()
}

pub async fn main() -> anyhow::Result<()> {
  let env = envy::from_env::<Env>()?;

  let restore_folder =
    if let Some(restore_folder) = env.komodo_restore_folder {
      env.komodo_backup_folder.join(&restore_folder)
    } else {
      latest_restore_folder(&env).await?
    }
    .components()
    .collect::<PathBuf>();

  info!("Restore folder: {restore_folder:?}");

  let mut restore_dir = tokio::fs::read_dir(&restore_folder)
    .await
    .with_context(|| {
      format!("Failed to read restore directory {restore_folder:?}")
    })?;

  let mut restore_files: Vec<(String, PathBuf)> = vec![(
    String::from("Stats"),
    env
      .komodo_backup_folder
      .join("Stats.jsonl.gz")
      .components()
      .collect(),
  )];

  loop {
    match restore_dir
      .next_entry()
      .await
      .context("Failed to read restore dir entry")
    {
      Ok(Some(file)) => {
        let path = file.path();
        let Some(file_name) = path.file_name() else {
          continue;
        };
        let Some(file_name) = file_name.to_str() else {
          continue;
        };
        let Some(collection) = file_name.strip_suffix(".jsonl.gz")
        else {
          continue;
        };
        restore_files.push((
          collection.to_string(),
          path.components().collect(),
        ));
      }
      Ok(None) => break,
      Err(e) => {
        warn!("{e:#}");
        continue;
      }
    }
  }

  // info!("Restoring: {restore_files:#?}");

  let mut db_builder = MongoBuilder::default();
  if let Some(uri) = maybe_read_item_from_file(
    env.komodo_database_uri_file,
    env.komodo_database_uri,
  ) {
    db_builder = db_builder.uri(uri);
  }
  if let Some(address) = env.komodo_database_address {
    db_builder = db_builder.address(address);
  }
  if let Some(username) = maybe_read_item_from_file(
    env.komodo_database_username_file,
    env.komodo_database_username,
  ) {
    db_builder = db_builder.username(username);
  }
  if let Some(password) = maybe_read_item_from_file(
    env.komodo_database_password_file,
    env.komodo_database_password,
  ) {
    db_builder = db_builder.password(password);
  }
  let target_db = db_builder
    .app_name(env.komodo_database_app_name)
    .build()
    .await
    .context("Failed to initialize target database")?
    .database(&env.komodo_database_db_name);

  let mut handles = restore_files
    .into_iter()
    .map(|(collection, restore_file)| {
      let target =
        target_db.collection::<Document>(&collection);

      async {
        let col = collection.clone();
        tokio::join!(
          async { col },
          tokio::spawn(async move {
            let res = async {
              let mut buffer = Vec::<Document>::new();
              let mut count = 0;

              let file = tokio::fs::File::open(&restore_file)
                .await
                .with_context(|| format!("Failed to open file {restore_file:?}"))?;

              let mut reader = tokio_util::codec::FramedRead::new(
                GzipDecoder::new(BufReader::new(file)),
                tokio_util::codec::LinesCodec::new()
              );

              while let Some(line) = reader.try_next()
                .await
                .context("Failed to get next line")?
              {
                let line = line.trim();
                if line.is_empty() {
                  continue;
                }
                let doc = match serde_json::from_str::<Document>(line)
                  .context("Failed to deserialize line to document")
                {
                  Ok(doc) => doc,
                  Err(e) => {
                    warn!("{e:#} | {line}");
                    continue;
                  }
                };
                count += 1;
                buffer.push(doc);
                if buffer.len() >= 20_000 {
                  if let Err(e) = target
                    .insert_many(&buffer)
                    .with_options(
                      InsertManyOptions::builder().ordered(false).build(),
                    )
                    .await
                  {
                    error!("Failed to flush document batch in {collection} collection | {e:#}");
                  };
                  buffer.clear();
                }
              }
              if !buffer.is_empty() {
                target
                  .insert_many(&buffer)
                  .with_options(
                    InsertManyOptions::builder().ordered(false).build(),
                  )
                  .await
                  .context("Failed to flush documents")?;
              }
              anyhow::Ok(count)
            }.await;
            match res {
              Ok(count) => {
                if count > 0 {
                  info!("Finished restoring {collection} collection | Restored {count}");
                }
              }
              Err(e) => {
                error!("Failed to restore {collection} collection | {e:#}")
              }
            }
          })
        )
      }
    })
    .collect::<FuturesUnordered<_>>();

  loop {
    match handles.next().await {
      Some((_collection, Ok(()))) => {
        // info!("[{collection}]: finished");
      }
      Some((collection, Err(e))) => {
        error!("[{collection}]: {e:#}");
      }
      None => break,
    }
  }

  info!("Finished restoring database ✅");

  Ok(())
}

async fn latest_restore_folder(env: &Env) -> anyhow::Result<PathBuf> {
  let mut max = PathBuf::new();
  let mut backups_dir =
    tokio::fs::read_dir(&env.komodo_backup_folder)
      .await
      .context("Failed to read restore directory")?;
  loop {
    match backups_dir
      .next_entry()
      .await
      .context("Failed to read dir entry")
    {
      Ok(Some(entry)) => {
        let path = entry.path();
        if path.is_dir() && path > max {
          max = path;
        }
      }
      Ok(None) => break,
      Err(e) => {
        warn!("{e:#}");
        continue;
      }
    }
  }
  Ok(max.components().collect())
}
