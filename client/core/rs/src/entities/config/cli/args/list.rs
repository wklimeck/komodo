#[derive(Debug, Clone, clap::Parser)]
pub struct List {
  /// List specific resources
  #[command(subcommand)]
  pub command: Option<ListCommand>,
  /// List all resources, including down ones.
  #[arg(long, short = 'a', default_value_t = false)]
  pub all: bool,
  /// Reverse the ordering of results,
  /// so non-running containers are listed first if --all is passed.
  #[arg(long, short = 'r', default_value_t = false)]
  pub reverse: bool,
  /// List only non-running / non-ok resources.
  #[arg(long, short = 'd', default_value_t = false)]
  pub down: bool,
  /// Filter by a particular name. Supports wildcard.
  /// Can be specified multiple times. (alias `n`)
  #[arg(name = "name", long, short = 'n')]
  pub names: Vec<String>,
  /// Filter by a particular tag.
  /// Can be specified multiple times. (alias `t`)
  #[arg(name = "tag", long, short = 't')]
  pub tags: Vec<String>,
  /// Filter by a particular server. Supports wildcard.
  /// Can be specified multiple times. (alias `s`)
  #[arg(name = "server", long, short = 's')]
  pub servers: Vec<String>,
  /// Filter by a particular builder. Supports wildcard.
  /// Can be specified multiple times. (alias `b`)
  #[arg(name = "builder", long, short = 'b')]
  pub builders: Vec<String>,
  /// Specify the format of the output.
  #[arg(long, short = 'f', default_value_t = super::CliFormat::Table)]
  pub format: super::CliFormat,
}

impl From<List> for ResourceFilters {
  fn from(value: List) -> Self {
    Self {
      all: value.all,
      reverse: value.reverse,
      down: value.down,
      names: value.names,
      tags: value.tags,
      servers: value.servers,
      builders: value.builders,
      format: value.format,
    }
  }
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ListCommand {
  /// List Servers (aliases: `server`, `srv`)
  #[clap(alias = "server", alias = "srv")]
  Servers(ResourceFilters),
  /// List Stacks (aliases: `stack`, `stk`)
  #[clap(alias = "stack", alias = "stk")]
  Stacks(ResourceFilters),
  /// List Deployments (aliases: `deployment`, `dep`)
  #[clap(alias = "deployment", alias = "dep")]
  Deployments(ResourceFilters),
  /// List Builds (aliases: `build`, `bld`)
  #[clap(alias = "build", alias = "bld")]
  Builds(ResourceFilters),
  /// List Repos (alias: `repo`)
  #[clap(alias = "repo")]
  Repos(ResourceFilters),
  /// List Procedures (aliases: `procedure`, `pr`)
  #[clap(alias = "procedure", alias = "pr")]
  Procedures(ResourceFilters),
  /// List Actions (aliases: `action`, `act`)
  #[clap(alias = "action", alias = "act")]
  Actions(ResourceFilters),
  /// List Syncs (alias: `sync`)
  #[clap(alias = "sync")]
  Syncs(ResourceFilters),
  /// List Builders (aliases: `builder`, `bldr`)
  #[clap(alias = "builder", alias = "bldr")]
  Builders(ResourceFilters),
  /// List Alerters (aliases: `alerter`, `alrt`)
  #[clap(alias = "alerter", alias = "alrt")]
  Alerters(ResourceFilters),
}

#[derive(Debug, Clone, clap::Parser)]
pub struct ResourceFilters {
  /// List all resources, including down ones.
  #[arg(long, short = 'a', default_value_t = false)]
  pub all: bool,
  /// Reverse the ordering of results,
  /// so non-running containers are listed first if --all is passed.
  #[arg(long, short = 'r', default_value_t = false)]
  pub reverse: bool,
  /// List only non-running / non-ok resources.
  #[arg(long, short = 'd', default_value_t = false)]
  pub down: bool,
  /// Filter by a particular name. Supports wildcard.
  /// Can be specified multiple times. (alias `n`)
  #[arg(name = "name", long, short = 'n')]
  pub names: Vec<String>,
  /// Filter by a particular tag.
  /// Can be specified multiple times. (alias `t`)
  #[arg(name = "tag", long, short = 't')]
  pub tags: Vec<String>,
  /// Filter by a particular server. Supports wildcard.
  /// Can be specified multiple times. (alias `s`)
  #[arg(name = "server", long, short = 's')]
  pub servers: Vec<String>,
  /// Filter by a particular builder. Supports wildcard.
  /// Can be specified multiple times. (alias `b`)
  #[arg(name = "builder", long, short = 'b')]
  pub builders: Vec<String>,
  /// Specify the format of the output.
  #[arg(long, short = 'f', default_value_t = super::CliFormat::Table)]
  pub format: super::CliFormat,
}
