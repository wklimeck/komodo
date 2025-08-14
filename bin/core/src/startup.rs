use std::str::FromStr;

use database::mungos::{
  find::find_collect,
  mongodb::bson::{Document, doc, oid::ObjectId, to_document},
};
use futures::future::join_all;
use komodo_client::{
  api::{
    execute::{BackupCoreDatabase, Execution, RunAction},
    write::{CreateBuilder, CreateServer},
  },
  entities::{
    ResourceTarget,
    action::Action,
    builder::{PartialBuilderConfig, PartialServerBuilderConfig},
    komodo_timestamp,
    procedure::{
      EnabledExecution, Procedure, ProcedureConfig, ProcedureStage,
    },
    server::{PartialServerConfig, Server},
    sync::ResourceSync,
    update::Log,
    user::{action_user, system_user},
  },
};
use resolver_api::Resolve;

use crate::{
  api::execute::{ExecuteArgs, ExecuteRequest},
  api::write::WriteArgs,
  config::core_config,
  helpers::random_string,
  helpers::update::init_execution_update,
  network, resource,
  state::db_client,
};

/// Runs the Actions with `run_at_startup: true`
pub async fn run_startup_actions() {
  let startup_actions =
    match database::mungos::find::find_collect::<Action>(
      &db_client().actions,
      doc! { "config.run_at_startup": true },
      None,
    )
    .await
    {
      Ok(actions) => actions,
      Err(e) => {
        error!("Failed to fetch actions for startup | {e:#?}");
        return;
      }
    };

  for action in startup_actions {
    let name = action.name;
    let id = action.id;
    let update = match init_execution_update(
      &ExecuteRequest::RunAction(RunAction {
        action: name.clone(),
        args: Default::default(),
      }),
      action_user(),
    )
    .await
    {
      Ok(update) => update,
      Err(e) => {
        error!(
          "Failed to initialize update for action {name} ({id}) | {e:#?}"
        );
        continue;
      }
    };

    if let Err(e) = (RunAction {
      action: name.clone(),
      args: Default::default(),
    })
    .resolve(&ExecuteArgs {
      user: action_user().to_owned(),
      update,
    })
    .await
    {
      error!(
        "Failed to execute startup action {name} ({id}) | {e:#?}"
      );
    }
  }
}

/// This function should be run on startup,
/// after the db client has been initialized
pub async fn on_startup() {
  // Configure manual network interface if specified
  network::configure_internet_gateway().await;

  tokio::join!(
    in_progress_update_cleanup(),
    open_alert_cleanup(),
    clean_up_server_templates(),
    ensure_first_server_and_builder(),
    ensure_default_procedures(),
  );
}

async fn in_progress_update_cleanup() {
  let log = Log::error(
    "Komodo shutdown",
    String::from(
      "Komodo shutdown during execution. If this is a build, the builder may not have been terminated.",
    ),
  );
  // This static log won't fail to serialize, unwrap ok.
  let log = to_document(&log).unwrap();
  if let Err(e) = db_client()
    .updates
    .update_many(
      doc! { "status": "InProgress" },
      doc! {
        "$set": {
          "status": "Complete",
          "success": false,
        },
        "$push": {
          "logs": log
        }
      },
    )
    .await
  {
    error!("failed to cleanup in progress updates on startup | {e:#}")
  }
}

/// Run on startup, ensure open alerts pointing to invalid resources are closed.
async fn open_alert_cleanup() {
  let db = db_client();
  let Ok(alerts) =
    find_collect(&db.alerts, doc! { "resolved": false }, None)
      .await
      .inspect_err(|e| {
        error!(
          "failed to list all alerts for startup open alert cleanup | {e:?}"
        )
      })
  else {
    return;
  };
  let futures = alerts.into_iter().map(|alert| async move {
    match alert.target {
      ResourceTarget::Server(id) => {
        resource::get::<Server>(&id)
          .await
          .is_err()
          .then(|| ObjectId::from_str(&alert.id).inspect_err(|e| warn!("failed to clean up alert - id is invalid ObjectId | {e:?}")).ok()).flatten()
      }
      ResourceTarget::ResourceSync(id) => {
        resource::get::<ResourceSync>(&id)
          .await
          .is_err()
          .then(|| ObjectId::from_str(&alert.id).inspect_err(|e| warn!("failed to clean up alert - id is invalid ObjectId | {e:?}")).ok()).flatten()
      }
      // No other resources should have open alerts.
      _ => ObjectId::from_str(&alert.id).inspect_err(|e| warn!("failed to clean up alert - id is invalid ObjectId | {e:?}")).ok(),
    }
  });
  let to_update_ids = join_all(futures)
    .await
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
  if let Err(e) = db
    .alerts
    .update_many(
      doc! { "_id": { "$in": to_update_ids } },
      doc! { "$set": {
        "resolved": true,
        "resolved_ts": komodo_timestamp()
      } },
    )
    .await
  {
    error!(
      "failed to clean up invalid open alerts on startup | {e:#}"
    )
  }
}

/// Ensures a default server / builder exists with the defined address
async fn ensure_first_server_and_builder() {
  let first_server = &core_config().first_server;
  if first_server.is_empty() {
    return;
  }
  let db = db_client();
  let Ok(server) = db
    .servers
    .find_one(Document::new())
    .await
    .inspect_err(|e| error!("Failed to initialize 'first_server'. Failed to query db. {e:?}"))
  else {
    return;
  };
  let server = if let Some(server) = server {
    server
  } else {
    match (CreateServer {
      name: format!("server-{}", random_string(5)),
      config: PartialServerConfig {
        address: Some(first_server.to_string()),
        enabled: Some(true),
        ..Default::default()
      },
    })
    .resolve(&WriteArgs {
      user: system_user().to_owned(),
    })
    .await
    {
      Ok(server) => server,
      Err(e) => {
        error!(
          "Failed to initialize 'first_server'. Failed to CreateServer. {:#}",
          e.error
        );
        return;
      }
    }
  };
  let Ok(None) = db.builders
    .find_one(Document::new()).await
    .inspect_err(|e| error!("Failed to initialize 'first_builder' | Failed to query db | {e:?}")) else {
      return;
    };
  if let Err(e) = (CreateBuilder {
    name: String::from("local"),
    config: PartialBuilderConfig::Server(
      PartialServerBuilderConfig {
        server_id: Some(server.id),
      },
    ),
  })
  .resolve(&WriteArgs {
    user: system_user().to_owned(),
  })
  .await
  {
    error!(
      "Failed to initialize 'first_builder' | Failed to CreateBuilder | {:#}",
      e.error
    );
  }
}

async fn ensure_default_procedures() {
  let db = db_client();
  // Assumes if there are any existing users or procedures,
  // the default procedures do not need to be set up.
  let Ok((None, None)) = tokio::try_join!(
    db.users.find_one(Document::new()),
    db.procedures.find_one(Document::new()),
  ).inspect_err(|e| error!("Failed to initialize default procedures | Failed to query db | {e:?}")) else {
    return
  };
  let Ok(config) = ProcedureConfig::builder()
    .stages(vec![ProcedureStage { 
      name: String::from("Backup"),
      enabled: true,
      executions: vec![
        EnabledExecution {
          execution: Execution::BackupCoreDatabase(BackupCoreDatabase {}),
          enabled: true
        }
      ]
    }])
    .schedule(String::from("Every day at 01:00"))
    .build()
    .inspect_err(|e| error!("Failed to initialize default procedures | Failed to build Procedure | {e:?}")) else {
    return;
  };
  let db_backup = Procedure {
    id: String::new(),
    name: String::from("Backup Core Database"),
    description: String::from(
      "Triggers the Core database backup at the scheduled time. Feel free to modify this to your needs, it will only be created with a fresh Komodo.",
    ),
    updated_at: komodo_timestamp(),
    config,
    template: Default::default(),
    tags: Default::default(),
    info: Default::default(),
    base_permission: Default::default(),
  };
  if let Err(e) = db.procedures.insert_one(&db_backup).await {
    error!("Failed to initialize default database backup Procedure | Failed to build Procedure | {e:?}");
  };
}

// LEGACY

/// v1.17.5 removes the ServerTemplate resource.
/// References to this resource type need to be cleaned up
/// to avoid type errors reading from the database.
async fn clean_up_server_templates() {
  let db = db_client();
  tokio::join!(
    async {
      db.permissions
        .delete_many(doc! {
          "resource_target.type": "ServerTemplate",
        })
        .await
        .expect(
          "Failed to clean up server template permissions on db",
        );
    },
    async {
      db.updates
        .delete_many(doc! { "target.type": "ServerTemplate" })
        .await
        .expect("Failed to clean up server template updates on db");
    },
    async {
      db.users
        .update_many(
          Document::new(),
          doc! { "$unset": { "recents.ServerTemplate": 1, "all.ServerTemplate": 1 } }
        )
        .await
        .expect("Failed to clean up server template updates on db");
    },
    async {
      db.user_groups
        .update_many(
          Document::new(),
          doc! { "$unset": { "all.ServerTemplate": 1 } },
        )
        .await
        .expect("Failed to clean up server template updates on db");
    },
  );
}
