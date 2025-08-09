use komodo_client::entities::{
  build::PartialBuildConfig, config::cli::UpdateCommand,
  deployment::PartialDeploymentConfig, repo::PartialRepoConfig,
  server::PartialServerConfig, stack::PartialStackConfig,
  sync::PartialResourceSyncConfig,
};

mod resource;
mod user;
mod variable;

pub async fn handle(command: &UpdateCommand) -> anyhow::Result<()> {
  match command {
    UpdateCommand::Build { build, update, yes } => {
      resource::update::<PartialBuildConfig>(build, update, *yes)
        .await
    }
    UpdateCommand::Deployment {
      deployment,
      update,
      yes,
    } => {
      resource::update::<PartialDeploymentConfig>(
        deployment, update, *yes,
      )
      .await
    }
    UpdateCommand::Repo { repo, update, yes } => {
      resource::update::<PartialRepoConfig>(repo, update, *yes).await
    }
    UpdateCommand::Server {
      server,
      update,
      yes,
    } => {
      resource::update::<PartialServerConfig>(server, update, *yes)
        .await
    }
    UpdateCommand::Stack { stack, update, yes } => {
      resource::update::<PartialStackConfig>(stack, update, *yes)
        .await
    }
    UpdateCommand::Sync { sync, update, yes } => {
      resource::update::<PartialResourceSyncConfig>(
        sync, update, *yes,
      )
      .await
    }
    UpdateCommand::Variable {
      name,
      value,
      secret,
      yes,
    } => variable::update(name, value, *secret, *yes).await,
    UpdateCommand::User { username, command } => {
      user::update(username, command).await
    }
  }
}
