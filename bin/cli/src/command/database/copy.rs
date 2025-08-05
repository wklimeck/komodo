use komodo_client::entities::config::core::DatabaseConfig;

use crate::config::cli_config;

pub async fn copy(target_uri: Option<String>) -> anyhow::Result<()> {
  let config = cli_config();

  let source_db = database::init(&config.database).await?;
  // let target = database::init(&DatabaseConfig {
  //   uri: 
  // })
  Ok(())
}
