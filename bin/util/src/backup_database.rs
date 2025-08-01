use std::{path::PathBuf, str::FromStr, time::Duration};

use anyhow::Context;
use async_compression::tokio::write::GzipEncoder;
// use async_compression::tokio::write::GzipEncoder;
use chrono::Local;
use environment_file::maybe_read_item_from_file;
use futures_util::{
  SinkExt, StreamExt, TryStreamExt, stream::FuturesUnordered,
};
use mungos::{
  init::MongoBuilder,
  mongodb::bson::{Document, RawDocumentBuf},
};
use serde::Deserialize;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::codec::{FramedWrite, LinesCodec};

#[derive(Deserialize)]
struct Env {
  /// The root folder to store timestamped backup folders in.
  #[serde(default = "default_backup_folder")]
  komodo_backup_folder: PathBuf,
  /// Give the source database some time to initialize.
  /// Default: 0
  #[serde(default)]
  startup_sleep_seconds: u64,

  #[serde(alias = "komodo_mongo_uri")]
  komodo_database_uri: Option<String>,

  #[serde(alias = "komodo_mongo_uri_file")]
  komodo_database_uri_file: Option<PathBuf>,

  #[serde(alias = "komodo_mongo_address")]
  komodo_database_address: Option<String>,

  #[serde(alias = "komodo_mongo_username")]
  komodo_database_username: Option<String>,

  #[serde(alias = "komodo_mongo_username_file")]
  komodo_database_username_file: Option<PathBuf>,

  #[serde(alias = "komodo_mongo_password")]
  komodo_database_password: Option<String>,

  #[serde(alias = "komodo_mongo_password_file")]
  komodo_database_password_file: Option<PathBuf>,

  #[serde(
    default = "default_app_name",
    alias = "komodo_mongo_app_name"
  )]
  komodo_database_app_name: String,

  #[serde(
    default = "default_db_name",
    alias = "komodo_mongo_db_name"
  )]
  komodo_database_db_name: String,
}

fn default_app_name() -> String {
  String::from("komodo-backup")
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

  if env.startup_sleep_seconds > 0 {
    info!("Sleeping for {} seconds...", env.startup_sleep_seconds);
    tokio::time::sleep(Duration::from_secs(
      env.startup_sleep_seconds,
    ))
    .await;
  }

  let now_backup_folder = env
    .komodo_backup_folder
    .join(Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());

  tokio::fs::create_dir_all(&now_backup_folder)
    .await
    .context("Failed to create backup folder")?;

  info!("Backing up to {now_backup_folder:?}");

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
  let source_db = db_builder
    .app_name(env.komodo_database_app_name)
    .build()
    .await
    .context("Failed to initialize source database")?
    .database(&env.komodo_database_db_name);

  let mut handles = source_db
    .list_collection_names()
    .await
    .context("Failed to list collections on source db")?.into_iter().map(|collection| {
      let source = source_db.collection::<RawDocumentBuf>(&collection);
      let file_path = if collection == "Stats" {
        env.komodo_backup_folder.join("Stats.jsonl.gz")
      } else {
        now_backup_folder.join(format!("{collection}.jsonl.gz"))
      };
      tokio::spawn(async move {
        let res = async {
          let mut count = 0;
          let _ = tokio::fs::remove_file(&file_path).await;
          let file = tokio::fs::File::create(&file_path)
            .await
            .with_context(|| format!("Failed to create file at {file_path:?}"))?;
          let mut writer = FramedWrite::new(
            BufWriter::new(GzipEncoder::with_quality(
              file, async_compression::Level::Best
            )),
            LinesCodec::new()
          );
          let mut cursor = source
            .find(Document::new())
            .await
            .context("Failed to query source collection")?;
          while let Some(doc) = cursor
            .try_next()
            .await
            .context("Failed to get next document")?
          {
            count += 1;
            let json = match serde_json::to_string(&doc).context("Failed to serialize document to json") {
              Ok(json) => json,
              Err(e) => {
                warn!("{e:#}");
                continue
              }
            };
            if let Err(e) = writer.send(json)
              .await
              .context("Failed to write document to file")
            {
              warn!("{e:#}");
            }
          }

          if let Err(e) = <_ as SinkExt<String>>::flush(&mut writer)
            .await
            .context("Failed to flush writer")
          {
            error!("{e:#}");
          };

          if let Err(e) = writer
            .into_inner()
            .shutdown()
            .await
            .context("Failed to shutdown writer compression")
          {
            error!("{e:#}");
          }

          anyhow::Ok(count)
        }
        .await;
        match res {
          Ok(count) => {
            if count > 0 {
              info!("Finished backing up {collection} collection | Backed up {count}");
            }
          }
          Err(e) => {
            error!("Failed to backup {collection} collection | {e:#}")
          }
        }
      })
    }).collect::<FuturesUnordered<_>>();

  loop {
    match handles.next().await {
      Some(Ok(())) => {}
      Some(Err(e)) => {
        error!("{e:#}");
      }
      None => break,
    }
  }

  info!("Finished backing up database ✅");

  Ok(())
}
