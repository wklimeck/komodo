use anyhow::Context;
use colored::Colorize;
use komodo_client::{
  api::{
    read::GetVariable,
    write::{
      CreateVariable, UpdateBuild, UpdateDeployment, UpdateRepo,
      UpdateResourceSync, UpdateServer, UpdateStack,
      UpdateVariableIsSecret, UpdateVariableValue,
    },
  },
  entities::{
    build::PartialBuildConfig, config::cli::UpdateCommand,
    deployment::PartialDeploymentConfig, repo::PartialRepoConfig,
    server::PartialServerConfig, stack::PartialStackConfig,
    sync::PartialResourceSyncConfig,
  },
};
use serde::{Serialize, de::DeserializeOwned};

pub async fn handle(command: &UpdateCommand) -> anyhow::Result<()> {
  match command {
    UpdateCommand::Variable {
      name,
      value,
      secret,
      yes,
    } => update_variable(name, value, *secret, *yes).await,
    UpdateCommand::Build { build, update, yes } => {
      update_resource::<PartialBuildConfig>(build, update, *yes).await
    }
    UpdateCommand::Deployment {
      deployment,
      update,
      yes,
    } => {
      update_resource::<PartialDeploymentConfig>(
        deployment, update, *yes,
      )
      .await
    }
    UpdateCommand::Repo { repo, update, yes } => {
      update_resource::<PartialRepoConfig>(repo, update, *yes).await
    }
    UpdateCommand::Server {
      server,
      update,
      yes,
    } => {
      update_resource::<PartialServerConfig>(server, update, *yes)
        .await
    }
    UpdateCommand::Stack { stack, update, yes } => {
      update_resource::<PartialStackConfig>(stack, update, *yes).await
    }
    UpdateCommand::Sync { sync, update, yes } => {
      update_resource::<PartialResourceSyncConfig>(sync, update, *yes)
        .await
    }
  }
}

async fn update_variable(
  name: &str,
  value: &str,
  secret: Option<bool>,
  yes: bool,
) -> anyhow::Result<()> {
  println!("\n{}: Update Variable\n", "Mode".dimmed());
  println!(" - {}:  {name}", "Name".dimmed());
  println!(" - {}: {value}", "Value".dimmed());
  if let Some(secret) = secret {
    println!(" - {}: {secret}", "Is Secret".dimmed());
  }

  super::wait_for_enter("update variable", yes)?;

  let client = super::komodo_client().await?;

  let Ok(existing) = client
    .read(GetVariable {
      name: name.to_string(),
    })
    .await
  else {
    // Create the variable
    client
      .write(CreateVariable {
        name: name.to_string(),
        value: value.to_string(),
        is_secret: secret.unwrap_or_default(),
        description: Default::default(),
      })
      .await
      .context("Failed to create variable")?;
    info!("Variable created ✅");
    return Ok(());
  };

  client
    .write(UpdateVariableValue {
      name: name.to_string(),
      value: value.to_string(),
    })
    .await
    .context("Failed to update variable 'value'")?;
  info!("Variable 'value' updated ✅");

  let Some(secret) = secret else { return Ok(()) };

  if secret != existing.is_secret {
    client
      .write(UpdateVariableIsSecret {
        name: name.to_string(),
        is_secret: secret,
      })
      .await
      .context("Failed to update variable 'is_secret'")?;
    info!("Variable 'is_secret' updated to {secret} ✅");
  }

  Ok(())
}

async fn update_resource<
  T: std::fmt::Debug + Serialize + DeserializeOwned + ResourceUpdate,
>(
  resource: &str,
  update: &str,
  yes: bool,
) -> anyhow::Result<()> {
  println!("\n{}: Update {}\n", "Mode".dimmed(), T::resource_type());
  println!(" - {}: {resource}", "Name".dimmed());

  let config = serde_qs::from_str::<T>(update)
    .context("Failed to deserialize config")?;

  match serde_json::to_string_pretty(&config) {
    Ok(config) => {
      println!(" - {}: {config}", "Update".dimmed());
    }
    Err(_) => {
      println!(" - {}: {config:#?}", "Update".dimmed());
    }
  }

  super::wait_for_enter("update resource", yes)?;

  config.apply(resource).await
}

trait ResourceUpdate {
  fn resource_type() -> &'static str;
  async fn apply(self, resource: &str) -> anyhow::Result<()>;
}

impl ResourceUpdate for PartialBuildConfig {
  fn resource_type() -> &'static str {
    "Build"
  }
  async fn apply(self, resource: &str) -> anyhow::Result<()> {
    let client = super::komodo_client().await?;
    client
      .write(UpdateBuild {
        id: resource.to_string(),
        config: self,
      })
      .await
      .context("Failed to update build config")?;
    Ok(())
  }
}

impl ResourceUpdate for PartialDeploymentConfig {
  fn resource_type() -> &'static str {
    "Deployment"
  }
  async fn apply(self, resource: &str) -> anyhow::Result<()> {
    let client = super::komodo_client().await?;
    client
      .write(UpdateDeployment {
        id: resource.to_string(),
        config: self,
      })
      .await
      .context("Failed to update deployment config")?;
    Ok(())
  }
}

impl ResourceUpdate for PartialRepoConfig {
  fn resource_type() -> &'static str {
    "Repo"
  }
  async fn apply(self, resource: &str) -> anyhow::Result<()> {
    let client = super::komodo_client().await?;
    client
      .write(UpdateRepo {
        id: resource.to_string(),
        config: self,
      })
      .await
      .context("Failed to update repo config")?;
    Ok(())
  }
}

impl ResourceUpdate for PartialServerConfig {
  fn resource_type() -> &'static str {
    "Server"
  }
  async fn apply(self, resource: &str) -> anyhow::Result<()> {
    let client = super::komodo_client().await?;
    client
      .write(UpdateServer {
        id: resource.to_string(),
        config: self,
      })
      .await
      .context("Failed to update server config")?;
    Ok(())
  }
}

impl ResourceUpdate for PartialStackConfig {
  fn resource_type() -> &'static str {
    "Stack"
  }
  async fn apply(self, resource: &str) -> anyhow::Result<()> {
    let client = super::komodo_client().await?;
    client
      .write(UpdateStack {
        id: resource.to_string(),
        config: self,
      })
      .await
      .context("Failed to update stack config")?;
    Ok(())
  }
}

impl ResourceUpdate for PartialResourceSyncConfig {
  fn resource_type() -> &'static str {
    "Sync"
  }
  async fn apply(self, resource: &str) -> anyhow::Result<()> {
    let client = super::komodo_client().await?;
    client
      .write(UpdateResourceSync {
        id: resource.to_string(),
        config: self,
      })
      .await
      .context("Failed to update sync config")?;
    Ok(())
  }
}
