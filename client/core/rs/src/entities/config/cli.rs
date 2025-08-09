use std::{path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{
  api::execute::Execution,
  deserializers::string_list_deserializer,
  entities::{
    config::{DatabaseConfig, empty_or_redacted},
    logger::{LogConfig, LogLevel, StdioLogMode},
  },
};

#[derive(Debug, clap::Parser)]
#[command(name = "komodo-cli", version, about = "", author)]
pub struct CliArgs {
  /// The command to run
  #[command(subcommand)]
  pub command: Command,

  /// Choose a custom [[profile]] name / alias set in a `komodo.cli.toml` file.
  #[arg(long, short = 'p')]
  pub profile: Option<String>,

  /// Sets the path of a config file or directory to use.
  /// Can use multiple times
  #[arg(long, short = 'c')]
  pub config_path: Option<Vec<PathBuf>>,

  /// Sets the keywords to match directory cli config file names on.
  /// Supports wildcard syntax.
  /// Can use multiple times to match multiple patterns independently.
  #[arg(long, short = 'm')]
  pub config_keyword: Option<Vec<String>>,

  /// Merges nested configs, eg. secrets, providers.
  /// Will override the equivalent env configuration.
  /// Default: true
  #[arg(long)]
  pub merge_nested_config: Option<bool>,

  /// Extends config arrays, eg. allowed_ips, passkeys.
  /// Will override the equivalent env configuration.
  /// Default: true
  #[arg(long)]
  pub extend_config_arrays: Option<bool>,

  /// Configure the logging level: error, warn, info, debug, trace.
  /// Default: info
  /// If passed, will override any other log_level set.
  #[arg(long, short = 'l')]
  pub log_level: Option<tracing::Level>,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Command {
  /// Print the CLI config being used. (alias: `cfg`)
  #[clap(alias = "cfg")]
  Config {
    /// Whether to print the additional profiles picked up
    #[arg(long, short = 'a', default_value_t = false)]
    all_profiles: bool,
    /// Whether to debug print the config
    #[arg(long, short = 'd', default_value_t = false)]
    debug: bool,
    /// Whether to print unsanitized config,
    /// including sensitive credentials.
    #[arg(long, action)]
    unsanitized: bool,
  },

  /// List containers and other resources (aliases: `ls`, `ps`)
  #[clap(alias = "ls", alias = "ps")]
  List(List),

  /// Run Komodo executions. (aliases: `x`, `run`, `deploy`, `dep`)
  #[clap(alias = "x", alias = "run", alias = "deploy", alias = "dep")]
  Execute {
    #[command(subcommand)]
    execution: Execution,
    /// Top priority Komodo host.
    /// Eg. "https://demo.komo.do"
    #[arg(long, short = 'a')]
    host: Option<String>,
    /// Top priority api key.
    #[arg(long, short = 'k')]
    key: Option<String>,
    /// Top priority api secret.
    #[arg(long, short = 's')]
    secret: Option<String>,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },

  /// Update resource configuration. (alias: `set`)
  #[clap(alias = "set")]
  Update {
    #[command(subcommand)]
    command: UpdateCommand,
  },

  /// Database utilities. (alias: `db`)
  #[clap(alias = "db")]
  Database {
    #[command(subcommand)]
    command: DatabaseCommand,
  },
}

#[derive(Debug, Clone, clap::Parser)]
pub struct List {
  /// List other Komodo entities
  #[command(subcommand)]
  pub command: Option<ListCommand>,
  /// List all containers, including stopped ones.
  #[arg(long, short = 'a', default_value_t = false)]
  pub all: bool,
  /// Reverse the ordering of when --all is passed,
  /// so non-running containers are listed first.
  #[arg(long, short = 'r', default_value_t = false)]
  pub reverse: bool,
  /// Filter containers by a particular server.
  /// Can be specified multiple times. (alias `s`)
  #[arg(name = "server", long, short = 's')]
  pub servers: Vec<String>,
  /// Filter containers by a name. Supports wildcard syntax.
  /// Can be specified multiple times. (alias `n`)
  #[arg(name = "name", long, short = 'n')]
  pub names: Vec<String>,
  /// Filter containers by image. Supports wildcard syntax.
  /// Can be specified multiple times. (alias `i`)
  #[arg(name = "image", long, short = 'i')]
  pub images: Vec<String>,
  /// Filter containers by image. Supports wildcard syntax.
  /// Can be specified multiple times. (alias `--net`)
  #[arg(name = "network", alias = "net", long)]
  pub networks: Vec<String>,
  /// Always continue on user confirmation prompts.
  #[arg(long, short = 'f', default_value_t = CliFormat::Table)]
  pub format: CliFormat,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ListCommand {
  #[clap(alias = "server", alias = "srv")]
  Servers,
  #[clap(alias = "stack", alias = "stk")]
  Stacks,
  #[clap(alias = "deployment", alias = "dep")]
  Deployments,
}

#[derive(
  Debug, Clone, Copy, Default, strum::Display, clap::ValueEnum,
)]
#[strum(serialize_all = "lowercase")]
pub enum CliFormat {
  /// Table output format. Default. (alias: `t`)
  #[default]
  #[clap(alias = "t")]
  Table,
  /// Json output format. (alias: `j`)
  #[clap(alias = "j")]
  Json,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum UpdateCommand {
  /// Update a Build's configuration. (alias: `bld`)
  #[clap(alias = "bld")]
  Build {
    /// The name / id of the Build.
    build: String,
    /// The update string, parsed using 'https://docs.rs/serde_qs/latest/serde_qs'.
    ///
    /// The fields can be found here: 'https://docs.rs/komodo_client/latest/komodo_client/entities/build/struct.BuildConfig.html'
    ///
    /// Example: `km update build example-build "version=1.13.4&branch=release"`
    ///
    /// Note. Should be enclosed in single or double quotes.
    /// Values containing complex characters (like URLs)
    /// will need to be url-encoded in order to be parsed correctly.
    update: String,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },

  /// Update a Deployments's configuration. (alias: `dep`)
  #[clap(alias = "dep")]
  Deployment {
    /// The name / id of the Deployment.
    deployment: String,
    /// The update string, parsed using 'https://docs.rs/serde_qs/latest/serde_qs'.
    ///
    /// The fields can be found here: 'https://docs.rs/komodo_client/latest/komodo_client/entities/deployment/struct.DeploymentConfig.html'
    ///
    /// Example: `km update deployment example-deployment "restart=unless-stopped"`
    ///
    /// Note. Should be enclosed in single or double quotes.
    /// Values containing complex characters (like URLs)
    /// will need to be url-encoded in order to be parsed correctly.
    update: String,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },

  /// Update a Repos's configuration.
  Repo {
    /// The name / id of the Repo.
    repo: String,
    /// The update string, parsed using 'https://docs.rs/serde_qs/latest/serde_qs'.
    ///
    /// The fields can be found here: 'https://docs.rs/komodo_client/latest/komodo_client/entities/repo/struct.RepoConfig.html'
    ///
    /// Example: `km update repo example-repo "branch=testing"`
    ///
    /// Note. Should be enclosed in single or double quotes.
    /// Values containing complex characters (like URLs)
    /// will need to be url-encoded in order to be parsed correctly.
    update: String,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },

  /// Update a Servers's configuration. (alias: `srv`)
  #[clap(alias = "srv")]
  Server {
    /// The name / id of the Server.
    server: String,
    /// The update string, parsed using 'https://docs.rs/serde_qs/latest/serde_qs'.
    ///
    /// The fields can be found here: 'https://docs.rs/komodo_client/latest/komodo_client/entities/server/struct.ServerConfig.html'
    ///
    /// Example: `km update server example-server "enabled=true&address=https%3A%2F%2Fmy.periphery%3A8120"`
    ///
    /// The above includes example of url encoded address `https://my.periphery:8120`.
    ///
    /// Note. Should be enclosed in single or double quotes.
    /// Values containing complex characters (like URLs)
    /// will need to be url-encoded in order to be parsed correctly.
    update: String,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },

  /// Update a Stacks's configuration. (alias: `stk`)
  #[clap(alias = "stk")]
  Stack {
    /// The name / id of the Stack.
    stack: String,
    /// The update string, parsed using 'https://docs.rs/serde_qs/latest/serde_qs'.
    ///
    /// The fields can be found here: 'https://docs.rs/komodo_client/latest/komodo_client/entities/stack/struct.StackConfig.html'
    ///
    /// Example: `km update stack example-stack "branch=testing"`
    ///
    /// Note. Should be enclosed in single or double quotes.
    /// Values containing complex characters (like URLs)
    /// will need to be url-encoded in order to be parsed correctly.
    update: String,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },

  /// Update a Syncs's configuration.
  Sync {
    /// The name / id of the Sync.
    sync: String,
    /// The update string, parsed using 'https://docs.rs/serde_qs/latest/serde_qs'.
    ///
    /// The fields can be found here: 'https://docs.rs/komodo_client/latest/komodo_client/entities/sync/struct.ResourceSyncConfig.html'
    ///
    /// Example: `km update sync example-sync "branch=testing"`
    ///
    /// Note. Should be enclosed in single or double quotes.
    /// Values containing complex characters (like URLs)
    /// will need to be url-encoded in order to be parsed correctly.
    update: String,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },

  /// Update a Variable's value. (alias: `var`)
  #[clap(alias = "var")]
  Variable {
    /// The name of the variable.
    name: String,
    /// The value to set variable to.
    value: String,
    /// Whether the value should be set to secret.
    /// If unset, will leave the variable secret setting as-is.
    #[arg(long, short = 's')]
    secret: Option<bool>,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },

  /// Update a user's configuration, including assigning resetting password and assigning Super Admin
  User {
    /// The user to update
    username: String,
    #[command(subcommand)]
    command: UpdateUserCommand,
  },
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum UpdateUserCommand {
  /// Update the users password. Fails if user is not "Local" user (ie OIDC). (alias: `pw`)
  #[clap(alias = "pw")]
  Password {
    /// The new password to use.
    password: String,
    /// Whether to print unsanitized config,
    /// including sensitive credentials.
    #[arg(long, action)]
    unsanitized: bool,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },
  /// Un/assign super admin to user. (aliases: `supa`, `sa`)
  #[clap(alias = "supa", alias = "sa")]
  SuperAdmin {
    #[clap(default_value_t = CliEnabled::Yes)]
    enabled: CliEnabled,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },
}

#[derive(
  Debug, Clone, Copy, Default, clap::ValueEnum, strum::Display,
)]
#[strum(serialize_all = "lowercase")]
pub enum CliEnabled {
  #[default]
  #[clap(alias = "y", alias = "true", alias = "t")]
  Yes,
  #[clap(alias = "n", alias = "false", alias = "f")]
  No,
}

impl From<CliEnabled> for bool {
  fn from(value: CliEnabled) -> Self {
    match value {
      CliEnabled::Yes => true,
      CliEnabled::No => false,
    }
  }
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum DatabaseCommand {
  /// Triggers database backup to compressed files
  /// organized by time the backup was taken. (alias: `bkp`)
  #[clap(alias = "bkp")]
  Backup {
    /// Optionally provide a specific backups folder.
    /// Default: `/backups`
    #[arg(long, short = 'f')]
    backups_folder: Option<PathBuf>,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },
  /// Restores the database from backup files. (alias: `rst`)
  #[clap(alias = "rst")]
  Restore {
    /// Optionally provide a specific backups folder.
    /// Default: `/backups`
    #[arg(long, short = 'f')]
    backups_folder: Option<PathBuf>,
    /// Optionally provide a specific restore folder.
    /// If not provided, will use the most recent backup folder.
    ///
    /// Example: `2025-08-01_05-04-53`
    #[arg(long, short = 'r')]
    restore_folder: Option<PathBuf>,
    /// Whether to index the target database. Default: true
    #[arg(long, short = 'i', default_value_t = true)]
    index: bool,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },
  /// Prunes database backups if there are greater than
  /// the configured `max_backups` (KOMODO_CLI_MAX_BACKUPS).
  Prune {
    /// Optionally provide a specific backups folder.
    /// Default: `/backups`
    #[arg(long, short = 'f')]
    backups_folder: Option<PathBuf>,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },
  /// Copy the database to another running database. (alias: `cp`)
  #[clap(alias = "cp")]
  Copy {
    /// The target database uri to copy to.
    #[arg(long)]
    uri: Option<String>,
    /// The target database address to copy to
    #[arg(long, short = 'a')]
    address: Option<String>,
    /// The target database username
    #[arg(long, short = 'u')]
    username: Option<String>,
    /// The target database password
    #[arg(long, short = 'p')]
    password: Option<String>,
    /// The target db name to copy to.
    #[arg(long, short = 'd')]
    db_name: Option<String>,
    /// Whether to index the target database. Default: true
    #[arg(long, short = 'i', default_value_t = true)]
    index: bool,
    /// Always continue on user confirmation prompts.
    #[arg(long, short = 'y', default_value_t = false)]
    yes: bool,
  },
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
  /// If not provided, will use "." (the current working directory).
  ///
  /// Note. This is overridden if the equivalent arg is passed in [CliArgs].
  #[serde(
    default = "default_config_paths",
    alias = "komodo_cli_config_path"
  )]
  pub komodo_cli_config_paths: Vec<PathBuf>,
  /// If specifying folders, use this to narrow down which
  /// files will be matched to parse into the final [CliConfig].
  /// Only files inside the folders which have names containing all keywords
  /// provided to `config_keywords` will be included.
  ///
  /// Note. This is overridden if the equivalent arg is passed in [CliArgs].
  #[serde(
    default = "default_config_keywords",
    alias = "komodo_cli_config_keyword"
  )]
  pub komodo_cli_config_keywords: Vec<String>,
  /// Will merge nested config object (eg. database) across multiple
  /// config files. Default: `true`
  ///
  /// Note. This is overridden if the equivalent arg is passed in [CliArgs].
  #[serde(default = "super::default_merge_nested_config")]
  pub komodo_cli_merge_nested_config: bool,
  /// Will extend config arrays (eg profiles) across multiple config files.
  /// Default: `true`
  ///
  /// Note. This is overridden if the equivalent arg is passed in [CliArgs].
  #[serde(default = "super::default_extend_config_arrays")]
  pub komodo_cli_extend_config_arrays: bool,
  // Override `default_profile`.
  pub komodo_cli_default_profile: Option<String>,
  /// Override `host` and `KOMODO_HOST`.
  pub komodo_cli_host: Option<String>,
  /// Override `cli_key`
  pub komodo_cli_key: Option<String>,
  /// Override `cli_secret`
  pub komodo_cli_secret: Option<String>,
  /// Override `backups_folder`
  pub komodo_cli_backups_folder: Option<PathBuf>,
  /// Override `max_backups`
  pub komodo_cli_max_backups: Option<u16>,
  /// Override `database_target_uri`
  #[serde(alias = "komodo_cli_database_copy_uri")]
  pub komodo_cli_database_target_uri: Option<String>,
  /// Override `database_target_address`
  #[serde(alias = "komodo_cli_database_copy_address")]
  pub komodo_cli_database_target_address: Option<String>,
  /// Override `database_target_username`
  #[serde(alias = "komodo_cli_database_copy_username")]
  pub komodo_cli_database_target_username: Option<String>,
  /// Override `database_target_password`
  #[serde(alias = "komodo_cli_database_copy_password")]
  pub komodo_cli_database_target_password: Option<String>,
  /// Override `database_target_db_name`
  #[serde(alias = "komodo_cli_database_copy_db_name")]
  pub komodo_cli_database_target_db_name: Option<String>,

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

fn default_config_paths() -> Vec<PathBuf> {
  if let Ok(home) = std::env::var("HOME") {
    vec![
      PathBuf::from_str(&home).unwrap().join(".config/komodo"),
      PathBuf::from_str(".").unwrap(),
    ]
  } else {
    vec![PathBuf::from_str(".").unwrap()]
  }
}

fn default_config_keywords() -> Vec<String> {
  vec![String::from("*komodo.cli*.*")]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
  /// Optional. Only relevant for top level CLI config.
  /// Set a default profile to be used when none is provided.
  /// This allows for quick switching between profiles while
  /// not having to explicitly pass `-p profile`.
  #[serde(
    alias = "default",
    skip_serializing_if = "Option::is_none"
  )]
  pub default_profile: Option<String>,
  /// Optional. The profile name. (alias: `name`)
  /// Configure profiles with name in the komodo.cli.toml,
  /// and select them using `km -p profile-name ...`.
  #[serde(
    default,
    alias = "name",
    skip_serializing_if = "String::is_empty"
  )]
  pub config_profile: String,
  /// Optional. The profile aliases. (aliases: `aliases`, `alias`)
  /// Configure profiles with alias in the komodo.cli.toml,
  /// and select them using `km -p alias ...`.
  #[serde(
    default,
    alias = "aliases",
    alias = "alias",
    deserialize_with = "string_list_deserializer",
    skip_serializing_if = "Vec::is_empty"
  )]
  pub config_aliases: Vec<String>,
  // Same as Core
  /// The host Komodo url.
  /// Eg. "https://demo.komo.do"
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub host: String,
  /// The api key for the CLI to use
  #[serde(alias = "key", skip_serializing_if = "Option::is_none")]
  pub cli_key: Option<String>,
  /// The api secret for the CLI to use
  #[serde(alias = "secret", skip_serializing_if = "Option::is_none")]
  pub cli_secret: Option<String>,
  /// The root backups folder.
  ///
  /// Default: `/backups`.
  ///
  /// Backups will be created in timestamped folders eg
  /// `/backups/2025-08-04_05_05_53`
  #[serde(default = "default_backups_folder")]
  pub backups_folder: PathBuf,

  /// Specify the maximum number of backups to keep,
  /// or 0 to disable backup pruning.
  /// Default: `14`
  ///
  /// After every backup, the CLI will prune the oldest backups
  /// if there are more backups than `max_backups`
  #[serde(default = "default_max_backups")]
  pub max_backups: u16,
  // Same as Core
  /// Configure database connection
  #[serde(
    default = "default_database_config",
    alias = "mongo",
    skip_serializing_if = "database_config_is_default"
  )]
  pub database: DatabaseConfig,
  /// Configure restore / copy database connection
  #[serde(
    default = "default_database_config",
    alias = "database_copy",
    skip_serializing_if = "database_config_is_default"
  )]
  pub database_target: DatabaseConfig,
  /// Logging configuration
  #[serde(
    default = "default_log_config",
    skip_serializing_if = "log_config_is_default"
  )]
  pub cli_logging: LogConfig,
  /// Configure additional profiles.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub profile: Vec<CliConfig>,
}

fn default_backups_folder() -> PathBuf {
  // SAFE: /backups is a valid path.
  PathBuf::from_str("/backups").unwrap()
}

fn default_max_backups() -> u16 {
  14
}

fn default_database_config() -> DatabaseConfig {
  DatabaseConfig {
    app_name: String::from("komodo_cli"),
    ..Default::default()
  }
}

fn database_config_is_default(db_config: &DatabaseConfig) -> bool {
  db_config == &default_database_config()
}

fn default_log_config() -> LogConfig {
  LogConfig {
    location: false,
    ..Default::default()
  }
}

fn log_config_is_default(log_config: &LogConfig) -> bool {
  log_config == &default_log_config()
}

impl Default for CliConfig {
  fn default() -> Self {
    Self {
      default_profile: Default::default(),
      config_profile: Default::default(),
      config_aliases: Default::default(),
      cli_key: Default::default(),
      cli_secret: Default::default(),
      cli_logging: default_log_config(),
      backups_folder: default_backups_folder(),
      max_backups: default_max_backups(),
      database: default_database_config(),
      database_target: default_database_config(),
      host: Default::default(),
      profile: Default::default(),
    }
  }
}

impl CliConfig {
  pub fn sanitized(&self) -> CliConfig {
    CliConfig {
      default_profile: self.default_profile.clone(),
      config_profile: self.config_profile.clone(),
      config_aliases: self.config_aliases.clone(),
      cli_key: self
        .cli_key
        .as_ref()
        .map(|cli_key| empty_or_redacted(cli_key)),
      cli_secret: self
        .cli_secret
        .as_ref()
        .map(|cli_secret| empty_or_redacted(cli_secret)),
      cli_logging: self.cli_logging.clone(),
      backups_folder: self.backups_folder.clone(),
      max_backups: self.max_backups,
      database_target: self.database_target.sanitized(),
      host: self.host.clone(),
      database: self.database.sanitized(),
      profile: self
        .profile
        .iter()
        .map(CliConfig::sanitized)
        .collect(),
    }
  }
}
