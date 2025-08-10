use std::collections::{HashMap, HashSet};

use anyhow::Context;
use colored::Colorize;
use comfy_table::{Attribute, Cell, Color};
use futures_util::{
  FutureExt, TryStreamExt, stream::FuturesUnordered,
};
use komodo_client::{
  api::read::{
    InspectDockerContainer, ListAllDockerContainers, ListServers,
  },
  entities::{
    config::cli::args::container::{
      Container, ContainerCommand, InspectContainer,
    },
    docker::container::{
      ContainerListItem, ContainerStateStatusEnum,
    },
  },
};

use crate::command::{
  PrintTable, matches_wildcards, parse_wildcards, print_items,
};

pub async fn handle(container: &Container) -> anyhow::Result<()> {
  match &container.command {
    None => list_containers(container).await,
    Some(ContainerCommand::Inspect(inspect)) => {
      inspect_container(inspect).await
    }
  }
}

async fn list_containers(
  Container {
    all,
    down,
    reverse,
    containers: names,
    images,
    networks,
    servers,
    format,
    command: _,
  }: &Container,
) -> anyhow::Result<()> {
  let client = super::komodo_client().await?;
  let (server_map, mut containers) = tokio::try_join!(
    client
      .read(ListServers::default())
      .map(|res| res.map(|res| res
        .into_iter()
        .map(|s| (s.id.clone(), s))
        .collect::<HashMap<_, _>>())),
    client.read(ListAllDockerContainers {
      servers: Default::default()
    }),
  )?;

  containers.iter_mut().for_each(|c| {
    let Some(server_id) = c.server_id.as_ref() else {
      return;
    };
    let Some(server) = server_map.get(server_id) else {
      c.server_id = Some(String::from("Unknown"));
      return;
    };
    c.server_id = Some(server.name.clone());
  });

  let names = parse_wildcards(names);
  let servers = parse_wildcards(servers);
  let images = parse_wildcards(images);
  let networks = parse_wildcards(networks);

  let mut containers = containers
    .into_iter()
    .filter(|c| {
      let state_check = if *all {
        true
      } else if *down {
        !matches!(c.state, ContainerStateStatusEnum::Running)
      } else {
        matches!(c.state, ContainerStateStatusEnum::Running)
      };
      let network_check = matches_wildcards(
        &networks,
        &c.network_mode
          .as_deref()
          .map(|n| vec![n])
          .unwrap_or_default(),
      ) || matches_wildcards(
        &networks,
        &c.networks.iter().map(String::as_str).collect::<Vec<_>>(),
      );
      state_check
        && network_check
        && matches_wildcards(&names, &[c.name.as_str()])
        && matches_wildcards(
          &servers,
          &c.server_id
            .as_deref()
            .map(|i| vec![i])
            .unwrap_or_default(),
        )
        && matches_wildcards(
          &images,
          &c.image.as_deref().map(|i| vec![i]).unwrap_or_default(),
        )
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

impl PrintTable for ContainerListItem {
  fn header() -> &'static [&'static str] {
    &["Container", "State", "Server", "Ports", "Networks", "Image"]
  }
  fn row(self) -> Vec<Cell> {
    let color = match self.state {
      ContainerStateStatusEnum::Running => Color::Green,
      ContainerStateStatusEnum::Paused => Color::DarkYellow,
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

pub async fn inspect_container(
  inspect: &InspectContainer,
) -> anyhow::Result<()> {
  let client = super::komodo_client().await?;
  let (server_map, mut containers) = tokio::try_join!(
    client
      .read(ListServers::default())
      .map(|res| res.map(|res| res
        .into_iter()
        .map(|s| (s.id.clone(), s))
        .collect::<HashMap<_, _>>())),
    client.read(ListAllDockerContainers {
      servers: Default::default()
    }),
  )?;

  containers.iter_mut().for_each(|c| {
    let Some(server_id) = c.server_id.as_ref() else {
      return;
    };
    let Some(server) = server_map.get(server_id) else {
      c.server_id = Some(String::from("Unknown"));
      return;
    };
    c.server_id = Some(server.name.clone());
  });

  let names = [inspect.container.to_string()];
  let names = parse_wildcards(&names);
  let servers = parse_wildcards(&inspect.servers);

  let mut containers = containers
    .into_iter()
    .filter(|c| {
      matches_wildcards(&names, &[c.name.as_str()])
        && matches_wildcards(
          &servers,
          &c.server_id
            .as_deref()
            .map(|i| vec![i])
            .unwrap_or_default(),
        )
    })
    .map(|c| async move {
      client
        .read(InspectDockerContainer {
          container: c.name,
          server: c.server_id.context("No server...")?,
        })
        .await
    })
    .collect::<FuturesUnordered<_>>()
    .try_collect::<Vec<_>>()
    .await?;

  containers.sort_by(|a, b| a.name.cmp(&b.name));

  match containers.len() {
    0 => {
      println!(
        "{}: Did not find any containers matching '{}'",
        "INFO".green(),
        inspect.container.bold()
      );
    }
    1 => {
      println!(
        "{}",
        serde_json::to_string_pretty(&containers[0])
          .context("Failed to serialize items to JSON")?
      );
    }
    _ => {
      println!(
        "{}",
        serde_json::to_string_pretty(&containers)
          .context("Failed to serialize items to JSON")?
      );
    }
  }

  Ok(())
}
