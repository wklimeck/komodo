use std::path::{Path, PathBuf};

use anyhow::Context;
use async_compression::tokio::bufread::GzipDecoder;
use futures_util::{
  StreamExt, TryStreamExt, stream::FuturesUnordered,
};
use mungos::mongodb::{
  Database, bson::Document, options::InsertManyOptions,
};
use tokio::io::BufReader;
use tokio_util::codec::{FramedRead, LinesCodec};
use tracing::{error, info, warn};

pub async fn restore(
  db: &Database,
  backup_folder: &Path,
  restore_folder: Option<&Path>,
) -> anyhow::Result<()> {
  // Get the specific dated folder to restore contents of
  let restore_folder = if let Some(restore_folder) = restore_folder {
    backup_folder.join(&restore_folder)
  } else {
    latest_restore_folder(backup_folder).await?
  }
  .components()
  .collect::<PathBuf>();

  info!("Restore folder: {restore_folder:?}");

  let restore_files =
    get_restore_files(backup_folder, &restore_folder).await?;

  let mut handles = restore_files
    .into_iter()
    .map(|(collection, restore_file)| {
      let target =
        db.collection::<Document>(&collection);

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

              let mut reader = FramedRead::new(
                GzipDecoder::new(BufReader::new(file)),
                LinesCodec::new()
              );

              while let Some(line) = reader.try_next()
                .await
                .context("Failed to get next line")?
              {
                if line.is_empty() {
                  continue;
                }
                let doc = match serde_json::from_str::<Document>(&line)
                  .context("Failed to deserialize line")
                {
                  Ok(doc) => doc,
                  Err(e) => {
                    warn!("{e:#}");
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
                  info!("[{collection}]: Restored {count} items");
                }
              }
              Err(e) => {
                error!("[{collection}]: {e:#}");
              }
            }
          })
        )
      }
    })
    .collect::<FuturesUnordered<_>>();

  loop {
    match handles.next().await {
      Some((_collection, Ok(()))) => {}
      Some((collection, Err(e))) => {
        error!("[{collection}]: {e:#}");
      }
      None => break,
    }
  }

  info!("Finished restoring database ✅");

  Ok(())
}

async fn latest_restore_folder(
  backup_folder: &Path,
) -> anyhow::Result<PathBuf> {
  let mut max = PathBuf::new();
  let mut backups_dir = tokio::fs::read_dir(backup_folder)
    .await
    .context("Failed to read backup directory")?;
  loop {
    match backups_dir
      .next_entry()
      .await
      .context("Failed to read backup dir entry")
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

async fn get_restore_files(
  backup_folder: &Path,
  restore_folder: &Path,
) -> anyhow::Result<Vec<(String, PathBuf)>> {
  let mut restore_dir =
    tokio::fs::read_dir(restore_folder).await.with_context(|| {
      format!("Failed to read restore directory {restore_folder:?}")
    })?;

  let mut restore_files: Vec<(String, PathBuf)> = vec![(
    String::from("Stats"),
    backup_folder.join("Stats.gz").components().collect(),
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
        let Some(collection) = file_name.strip_suffix(".gz") else {
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

  Ok(restore_files)
}
