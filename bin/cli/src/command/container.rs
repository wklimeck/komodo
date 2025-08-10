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

use crate::{
  command::{
    PrintTable, matches_wildcards, parse_wildcards, print_items,
    text_link,
  },
  config::cli_config,
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
  let (server_map, containers) = tokio::try_join!(
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

  // (Option<Server Name>, Container)
  let containers = containers.into_iter().map(|c| {
    let server = if let Some(server_id) = c.server_id.as_ref()
      && let Some(server) = server_map.get(server_id)
    {
      server
    } else {
      return (None, c);
    };
    (Some(server.name.as_str()), c)
  });

  let names = parse_wildcards(names);
  let servers = parse_wildcards(servers);
  let images = parse_wildcards(images);
  let networks = parse_wildcards(networks);

  let mut containers = containers
    .into_iter()
    .filter(|(server_name, c)| {
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
          &server_name
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
  containers.sort_by(|(a_s, a), (b_s, b)| {
    a.state
      .cmp(&b.state)
      .then(a.name.cmp(&b.name))
      .then(a_s.cmp(b_s))
      .then(a.network_mode.cmp(&b.network_mode))
      .then(a.image.cmp(&b.image))
  });
  if *reverse {
    containers.reverse();
  }
  print_items(containers, *format)?;
  Ok(())
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

// (Option<Server Name>, Container)
impl PrintTable for (Option<&'_ str>, ContainerListItem) {
  fn header() -> &'static [&'static str] {
    &["Container", "State", "Server", "Ports", "Networks", "Image"]
  }
  fn row(self) -> Vec<Cell> {
    let color = match self.1.state {
      ContainerStateStatusEnum::Running => Color::Green,
      ContainerStateStatusEnum::Paused => Color::DarkYellow,
      ContainerStateStatusEnum::Empty => Color::Grey,
      _ => Color::Red,
    };
    let mut networks = HashSet::new();
    if let Some(network) = self.1.network_mode {
      networks.insert(network);
    }
    for network in self.1.networks {
      networks.insert(network);
    }
    let mut networks = networks.into_iter().collect::<Vec<_>>();
    networks.sort();
    let mut ports = self
      .1
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
    let name = if let Some(server_id) = self.1.server_id {
      text_link(
        &format!(
          "{}/servers/{server_id}/container/{}",
          cli_config().host,
          self.1.name
        ),
        &self.1.name,
      )
    } else {
      self.1.name
    };
    vec![
      Cell::new(name).add_attribute(Attribute::Bold),
      Cell::new(self.1.state.to_string())
        .fg(color)
        .add_attribute(Attribute::Bold),
      Cell::new(self.0.unwrap_or("Unknown")),
      ports,
      Cell::new(networks.join(", ")),
      Cell::new(self.1.image.as_deref().unwrap_or("Unknown")),
    ]
  }
}
