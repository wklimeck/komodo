use anyhow::Context;
use futures_util::{
  StreamExt, TryStreamExt, stream::FuturesUnordered,
};
use mungos::mongodb::{
  Database,
  bson::{Document, RawDocumentBuf},
  options::InsertManyOptions,
};
use tracing::{error, info};

pub async fn copy(
  source_db: &Database,
  target_db: &Database,
) -> anyhow::Result<()> {
  let mut handles = source_db
    .list_collection_names()
    .await
    .context("Failed to list collections on source db")?.into_iter().map(|collection| {
      let source = source_db.collection::<RawDocumentBuf>(&collection);
      let target = target_db.collection::<RawDocumentBuf>(&collection);

      tokio::spawn(async move {
        let res = async {
          let mut buffer = Vec::<RawDocumentBuf>::new();
          let mut count = 0;
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
        }
        .await;
        match res {
          Ok(count) => {
            if count > 0 {
              info!("Finished copying {collection} collection | Copied {count}");
            }
          }
          Err(e) => {
            error!("Failed to copy {collection} collection | {e:#}")
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

  info!("Finished copying database ✅");

  Ok(())
}
