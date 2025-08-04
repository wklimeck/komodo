use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde::Deserialize;

use crate::{
  api::execute::Execution,
  entities::{
    config::core::DatabaseConfig,
    logger::{LogConfig, LogLevel, StdioLogMode},
  },
};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
  /// The command to run
  #[command(subcommand)]
  pub command: Command,

  /// Always continue on user confirmation prompts.
  #[arg(long, short, default_value_t = false)]
  pub yes: bool,

  /// Sets the path of a config file or directory to use.
  /// Can use multiple times
  #[arg(short, long)]
  pub config_path: Option<Vec<PathBuf>>,

  /// Sets the keywords to match directory cli config file names on.
  /// Supports wildcard syntax.
  /// Can use multiple times to match multiple patterns independently.
  #[arg(long)]
  pub config_keyword: Option<Vec<String>>,

  /// Merges nested configs, eg. secrets, providers.
  /// Will override the equivalent env configuration.
  /// Default: false
  #[arg(long)]
  pub merge_nested_config: Option<bool>,

  /// Extends config arrays, eg. allowed_ips, passkeys.
  /// Will override the equivalent env configuration.
  /// Default: false
  #[arg(long)]
  pub extend_config_arrays: Option<bool>,

  /// Top priority Komodo host.
  /// Eg. "https://demo.komo.do"
  #[arg(long)]
  pub host: Option<String>,
  /// Top priority api key.
  #[arg(long)]
  pub key: Option<String>,
  /// Top priority api secret.
  #[arg(long)]
  pub secret: Option<String>,

  /// Configure the logging level: error, warn, info, debug, trace.
  /// Default: info
  /// If passed, will override any other log_level set.
  #[arg(long)]
  pub log_level: Option<tracing::Level>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
  /// Run Komodo executions
  Execute {
    #[command(subcommand)]
    execution: Execution,
  },

  /// Database utilities
  Database {
    #[command(subcommand)]
    command: DatabaseCommand,
  },
}

#[derive(Debug, Clone, Subcommand)]
pub enum DatabaseCommand {
  /// Triggers database backup to compressed files
  /// organized by time the backup was taken.
  Backup,
  /// Restores the database from backup files.
  Restore {
    /// Optionally provide a specific restore folder time.
    /// If not provided, will use the most recent backup folder.
    /// Example: `2025-08-01_05-04-53`
    time: Option<String>,
  },
  /// Copy the database to another running database.
  Copy {
    /// Provide the target database uri to copy to.
    target_uri: Option<String>,
  }
}

/// # Komodo CLI Environment Variables
///
///
#[derive(Debug, Clone, Deserialize)]
pub struct Env {
  // ============
  // Cli specific
  // ============
  /// Specify the config paths (files or folders) used to build up the
  /// final [CliConfig].
  /// If not provided, will use Default config.
  ///
  /// Note. This is overridden if the equivalent arg is passed in [CliArgs].
  #[serde(default, alias = "komodo_cli_config_path")]
  pub komodo_cli_config_paths: Vec<PathBuf>,
  /// If specifying folders, use this to narrow down which
  /// files will be matched to parse into the final [CliConfig].
  /// Only files inside the folders which have names containing all keywords
  /// provided to `config_keywords` will be included.
  ///
  /// Note. This is overridden if the equivalent arg is passed in [CliArgs].
  #[serde(default, alias = "komodo_cli_config_keyword")]
  pub komodo_cli_config_keywords: Vec<String>,
  /// Will merge nested config object (eg. secrets, providers) across multiple
  /// config files. Default: `false`
  ///
  /// Note. This is overridden if the equivalent arg is passed in [CliArgs].
  #[serde(default = "default_merge_nested_config")]
  pub komodo_cli_merge_nested_config: bool,
  /// Will extend config arrays (eg. `allowed_ips`, `passkeys`) across multiple config files.
  /// Default: `false`
  ///
  /// Note. This is overridden if the equivalent arg is passed in [CliArgs].
  #[serde(default = "default_extend_config_arrays")]
  pub komodo_cli_extend_config_arrays: bool,
  /// Override `host` and `KOMODO_HOST`.
  pub komodo_cli_host: Option<String>,
  /// Override `cli_key`
  pub komodo_cli_key: Option<String>,
  /// Override `cli_secret`
  pub komodo_cli_secret: Option<String>,

  // LOGGING
  /// Override `logging.level`
  pub komodo_cli_logging_level: Option<LogLevel>,
  /// Override `logging.stdio`
  pub komodo_cli_logging_stdio: Option<StdioLogMode>,
  /// Override `logging.pretty`
  pub komodo_cli_logging_pretty: Option<bool>,
  /// Override `logging.otlp_endpoint`
  pub komodo_cli_logging_otlp_endpoint: Option<String>,
  /// Override `logging.opentelemetry_service_name`
  pub komodo_cli_logging_opentelemetry_service_name: Option<String>,
  /// Override `pretty_startup_config`
  pub komodo_cli_pretty_startup_config: Option<bool>,

  // ================
  // Same as Core env
  // ================
  /// Specify a custom config path for the core config toml.
  /// Used as a base for the `cli_config_paths`.
  /// Default: `/config/config.toml`
  #[serde(default = "super::default_config_path")]
  pub komodo_config_path: PathBuf,
  /// Override `host`
  pub komodo_host: Option<String>,

  // DATABASE
  /// Override `database.uri`
  #[serde(alias = "komodo_mongo_uri")]
  pub komodo_database_uri: Option<String>,
  /// Override `database.uri` from file
  #[serde(alias = "komodo_mongo_uri_file")]
  pub komodo_database_uri_file: Option<PathBuf>,
  /// Override `database.address`
  #[serde(alias = "komodo_mongo_address")]
  pub komodo_database_address: Option<String>,
  /// Override `database.username`
  #[serde(alias = "komodo_mongo_username")]
  pub komodo_database_username: Option<String>,
  /// Override `database.username` with file
  #[serde(alias = "komodo_mongo_username_file")]
  pub komodo_database_username_file: Option<PathBuf>,
  /// Override `database.password`
  #[serde(alias = "komodo_mongo_password")]
  pub komodo_database_password: Option<String>,
  /// Override `database.password` with file
  #[serde(alias = "komodo_mongo_password_file")]
  pub komodo_database_password_file: Option<PathBuf>,
  /// Override `database.db_name`
  #[serde(alias = "komodo_mongo_db_name")]
  pub komodo_database_db_name: Option<String>,
}

fn default_merge_nested_config() -> bool {
  true
}

fn default_extend_config_arrays() -> bool {
  true
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CliConfig {
  // ============
  // Cli specific
  // ============
  /// The api key for the CLI to use
  pub cli_key: Option<String>,
  /// The api secret for the CLI to use
  pub cli_secret: Option<String>,

  // ============
  // Same as Core
  // ============
  /// The host Komodo url.
  /// Eg. "https://demo.komo.do"
  #[serde(default)]
  pub host: String,
  /// Configure database connection
  #[serde(default, alias = "mongo")]
  pub database: DatabaseConfig,
  /// Logging configuration
  #[serde(default)]
  pub logging: LogConfig,
  /// Pretty-log (multi-line) the startup config
  /// for easier human readability.
  #[serde(default)]
  pub pretty_startup_config: bool,
}
