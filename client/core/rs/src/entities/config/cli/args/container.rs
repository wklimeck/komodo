#[derive(Debug, Clone, clap::Parser)]
pub struct Container {
  /// Other container utilities
  #[command(subcommand)]
  pub command: Option<ContainerCommand>,
  /// List all containers, including stopped ones.
  /// This overrides 'down'.
  #[arg(long, short = 'a', default_value_t = false)]
  pub all: bool,
  /// Reverse the ordering of results,
  /// so non-running containers are listed first if --all is passed.
  #[arg(long, short = 'r', default_value_t = false)]
  pub reverse: bool,
  /// List only non-running containers.
  #[arg(long, short = 'd', default_value_t = false)]
  pub down: bool,
  /// Filter containers by a particular server.
  /// Can be specified multiple times. (alias `s`)
  #[arg(name = "server", long, short = 's')]
  pub servers: Vec<String>,
  /// Filter containers by a name. Supports wildcard syntax.
  /// Can be specified multiple times. (alias `c`)
  #[arg(name = "container", long, short = 'c')]
  pub containers: Vec<String>,
  /// Filter containers by image. Supports wildcard syntax.
  /// Can be specified multiple times. (alias `i`)
  #[arg(name = "image", long, short = 'i')]
  pub images: Vec<String>,
  /// Filter containers by image. Supports wildcard syntax.
  /// Can be specified multiple times. (alias `--net`, `n`)
  #[arg(name = "network", alias = "net", long, short = 'n')]
  pub networks: Vec<String>,
  /// Specify the format of the output.
  #[arg(long, short = 'f', default_value_t = super::CliFormat::Table)]
  pub format: super::CliFormat,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ContainerCommand {}
