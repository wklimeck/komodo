use std::{path::PathBuf, sync::OnceLock};

use clap::Parser;
use environment_file::maybe_read_item_from_file;
use komodo_client::entities::{
  config::{
    cli::{CliArgs, CliConfig, Command, DatabaseCommand, Env},
    core::DatabaseConfig,
  },
  logger::{LogConfig, LogLevel},
};
use merge_config_files::parse_config_paths;

pub fn cli_args() -> &'static CliArgs {
  static CLI_ARGS: OnceLock<CliArgs> = OnceLock::new();
  CLI_ARGS.get_or_init(CliArgs::parse)
}

pub fn cli_config() -> &'static CliConfig {
  static CLI_CONFIG: OnceLock<CliConfig> = OnceLock::new();
  CLI_CONFIG.get_or_init(|| {
    let env: Env =
      envy::from_env().expect("failed to parse cli environment");
    let args = cli_args();
    let config_paths = args
      .config_path
      .clone()
      .unwrap_or(env.komodo_cli_config_paths);
    let config = if config_paths.is_empty() {
      CliConfig::default()
    } else {
      parse_config_paths::<CliConfig>(
        &config_paths
          .iter()
          .map(PathBuf::as_path)
          .collect::<Vec<_>>(),
        &args
          .config_keyword
          .clone()
          .unwrap_or(env.komodo_cli_config_keywords)
          .iter()
          .map(String::as_str)
          .collect::<Vec<_>>(),
        args
          .merge_nested_config
          .unwrap_or(env.komodo_cli_merge_nested_config),
        args
          .extend_config_arrays
          .unwrap_or(env.komodo_cli_extend_config_arrays),
      )
      .expect("failed at parsing config from paths")
    };

    let (host, key, secret) = match &args.command {
      Command::Execute {
        host, key, secret, ..
      } => (host.clone(), key.clone(), secret.clone()),
      _ => (None, None, None),
    };

    let (backup_folder, restore_folder) = match &args.command {
      Command::Database {
        command: DatabaseCommand::Backup { folder },
      } => (folder.clone(), None),
      Command::Database {
        command:
          DatabaseCommand::Restore {
            folder,
            backup_folder,
          },
      } => (backup_folder.clone(), folder.clone()),
      _ => (None, None),
    };
    let (uri, address, username, password, db_name) =
      match &args.command {
        Command::Database {
          command:
            DatabaseCommand::Copy {
              uri,
              address,
              username,
              password,
              db_name,
            },
        } => (
          uri.clone(),
          address.clone(),
          username.clone(),
          password.clone(),
          db_name.clone(),
        ),
        _ => (None, None, None, None, None),
      };

    CliConfig {
      host: host
        .or(env.komodo_cli_host)
        .or(env.komodo_host)
        .unwrap_or(config.host),
      cli_key: key.or(env.komodo_cli_key).or(config.cli_key),
      cli_secret: secret
        .or(env.komodo_cli_secret)
        .or(config.cli_secret),
      backup_folder: backup_folder
        .or(env.komodo_cli_backup_folder)
        .unwrap_or(config.backup_folder),
      restore_folder: restore_folder
        .or(env.komodo_cli_restore_folder)
        .or(config.restore_folder),
      database_copy: DatabaseConfig {
        uri: uri
          .or(env.komodo_cli_database_copy_uri)
          .unwrap_or(config.database_copy.uri),
        address: address
          .or(env.komodo_cli_database_copy_address)
          .unwrap_or(config.database_copy.address),
        username: username
          .or(env.komodo_cli_database_copy_username)
          .unwrap_or(config.database_copy.username),
        password: password
          .or(env.komodo_cli_database_copy_password)
          .unwrap_or(config.database_copy.password),
        db_name: db_name
          .or(env.komodo_cli_database_copy_db_name)
          .unwrap_or(config.database_copy.db_name),
        app_name: String::from("komodo_cli"),
      },
      database: DatabaseConfig {
        uri: maybe_read_item_from_file(
          env.komodo_database_uri_file,
          env.komodo_database_uri,
        )
        .unwrap_or(config.database.uri),
        address: env
          .komodo_database_address
          .unwrap_or(config.database.address),
        username: maybe_read_item_from_file(
          env.komodo_database_username_file,
          env.komodo_database_username,
        )
        .unwrap_or(config.database.username),
        password: maybe_read_item_from_file(
          env.komodo_database_password_file,
          env.komodo_database_password,
        )
        .unwrap_or(config.database.password),
        db_name: env
          .komodo_database_db_name
          .unwrap_or(config.database.db_name),
        app_name: String::from("komodo_cli"),
      },
      logging: LogConfig {
        level: args
          .log_level
          .map(LogLevel::from)
          .or(env.komodo_cli_logging_level)
          .unwrap_or(config.logging.level),
        stdio: env
          .komodo_cli_logging_stdio
          .unwrap_or(config.logging.stdio),
        pretty: env
          .komodo_cli_logging_pretty
          .unwrap_or(config.logging.pretty),
        otlp_endpoint: env
          .komodo_cli_logging_otlp_endpoint
          .unwrap_or(config.logging.otlp_endpoint),
        opentelemetry_service_name: env
          .komodo_cli_logging_opentelemetry_service_name
          .unwrap_or(config.logging.opentelemetry_service_name),
      },
      pretty_startup_config: env
        .komodo_cli_pretty_startup_config
        .unwrap_or(config.pretty_startup_config),
    }
  })
}
