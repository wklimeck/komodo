use std::collections::{HashMap, HashSet};

use anyhow::Context;
use comfy_table::{Attribute, Cell, Color, Table};
use futures_util::FutureExt;
use komodo_client::{
  api::read::{ListAllDockerContainers, ListServers},
  entities::{
    config::cli::{self, CliFormat},
    docker::container::{
      ContainerListItem, ContainerStateStatusEnum,
    },
    resource::ResourceQuery,
  },
};
use serde::Serialize;
use wildcard::Wildcard;

pub async fn handle(list: &cli::List) -> anyhow::Result<()> {
  match list.command {
    None => list_containers(list).await,
    Some(_) => Ok(()),
  }
}

async fn list_containers(
  cli::List {
    all,
    reverse,
    names,
    images,
    networks,
    servers,
    format,
    command: _,
  }: &cli::List,
) -> anyhow::Result<()> {
  let client = super::komodo_client().await?;
  let (servers, containers) = tokio::try_join!(
    client
      .read(ListServers {
        query: ResourceQuery::builder()
          .names(servers.clone())
          .build()
      })
      .map(|res| res.map(|res| res
        .into_iter()
        .map(|s| (s.id.clone(), s))
        .collect::<HashMap<_, _>>())),
    client.read(ListAllDockerContainers {
      servers: servers.clone()
    }),
  )?;

  let names = parse_wildcards(names);
  let images = parse_wildcards(images);
  let networks = parse_wildcards(networks);

  let mut containers = containers
    .into_iter()
    .filter(|c| {
      (*all || matches!(c.state, ContainerStateStatusEnum::Running))
        && matches_wildcards(&names, &[c.name.as_str()])
        && matches_wildcards(
          &images,
          &c.image.as_deref().map(|i| vec![i]).unwrap_or_default(),
        )
        && (matches_wildcards(
          &networks,
          &c.network_mode
            .as_deref()
            .map(|n| vec![n])
            .unwrap_or_default(),
        ) || matches_wildcards(
          &networks,
          &c.networks.iter().map(String::as_str).collect::<Vec<_>>(),
        ))
    })
    .map(|mut c| {
      let Some(server_id) = c.server_id.as_ref() else {
        return c;
      };
      let Some(server) = servers.get(server_id) else {
        c.server_id = Some(String::from("Unknown"));
        return c;
      };
      c.server_id = Some(server.name.clone());
      c
    })
    .collect::<Vec<_>>();
  containers.sort_by(|a, b| {
    a.state
      .cmp(&b.state)
      .then(a.name.cmp(&b.name))
      .then(a.server_id.cmp(&b.server_id))
      .then(a.network_mode.cmp(&b.network_mode))
      .then(a.image.cmp(&b.image))
  });
  if *reverse {
    containers.reverse();
  }
  print_items(containers, *format)?;
  Ok(())
}

fn print_items<T: PrintTable + Serialize>(
  items: Vec<T>,
  format: CliFormat,
) -> anyhow::Result<()> {
  match format {
    CliFormat::Table => {
      let mut table = Table::new();
      table.set_header(T::header());
      for item in items {
        table.add_row(item.row());
      }
      println!("{table}");
    }
    CliFormat::Json => {
      println!(
        "{}",
        serde_json::to_string_pretty(&items)
          .context("Failed to serialize items to JSON")?
      );
    }
  }
  Ok(())
}

trait PrintTable {
  fn header() -> &'static [&'static str];
  fn row(self) -> Vec<Cell>;
}

impl PrintTable for ContainerListItem {
  fn header() -> &'static [&'static str] {
    &["Container", "State", "Server", "Ports", "Networks", "Image"]
  }
  fn row(self) -> Vec<Cell> {
    let color = match self.state {
      ContainerStateStatusEnum::Running => Color::Green,
      ContainerStateStatusEnum::Paused => Color::Green,
      ContainerStateStatusEnum::Empty => Color::Grey,
      _ => Color::Red,
    };
    let mut networks = HashSet::new();
    if let Some(network) = self.network_mode {
      networks.insert(network);
    }
    for network in self.networks {
      networks.insert(network);
    }
    let mut networks = networks.into_iter().collect::<Vec<_>>();
    networks.sort();
    let mut ports = self
      .ports
      .into_iter()
      .flat_map(|p| p.public_port.map(|p| p.to_string()))
      .collect::<HashSet<_>>()
      .into_iter()
      .collect::<Vec<_>>();
    ports.sort();
    let ports = if ports.is_empty() {
      Cell::new("")
    } else {
      Cell::new(format!(":{}", ports.join(", :")))
    };
    vec![
      Cell::new(self.name).add_attribute(Attribute::Bold),
      Cell::new(self.state.to_string())
        .fg(color)
        .add_attribute(Attribute::Bold),
      Cell::new(self.server_id.unwrap_or(String::from("Unknown"))),
      ports,
      Cell::new(networks.join(", ")),
      Cell::new(self.image.unwrap_or(String::from("Unknown"))),
    ]
  }
}

fn parse_wildcards(items: &[String]) -> Vec<Wildcard<'_>> {
  items
    .iter()
    .flat_map(|i| {
      Wildcard::new(i.as_bytes()).inspect_err(|e| {
        warn!("Failed to parse wildcard: {i} | {e:?}")
      })
    })
    .collect::<Vec<_>>()
}

fn matches_wildcards(
  wildcards: &[Wildcard<'_>],
  items: &[&str],
) -> bool {
  if wildcards.is_empty() {
    return true;
  }
  items.iter().any(|item| {
    wildcards.iter().any(|wc| wc.is_match(item.as_bytes()))
  })
}
