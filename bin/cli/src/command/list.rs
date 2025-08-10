use std::collections::HashMap;

use comfy_table::{Attribute, Cell, Color};
use futures_util::{FutureExt, try_join};
use komodo_client::{
  KomodoClient,
  api::read::{ListServers, ListStacks, ListTags},
  entities::{
    config::cli::args::{
      self,
      list::{ListCommand, ResourceFilters},
    },
    resource::{ResourceListItem, ResourceQuery},
    server::{ServerListItem, ServerListItemInfo, ServerState},
    stack::{StackListItem, StackListItemInfo, StackState},
  },
};
use serde::Serialize;

use crate::command::{
  PrintTable, matches_wildcards, parse_wildcards, print_items,
};

pub async fn handle(list: &args::list::List) -> anyhow::Result<()> {
  match &list.command {
    None => list_all(list).await,
    Some(ListCommand::Servers(filters)) => {
      list_resources::<ServerListItem>(filters).await
    }
    Some(ListCommand::Stacks(filters)) => {
      list_resources::<StackListItem>(filters).await
    }
  }
}

async fn list_all(list: &args::list::List) -> anyhow::Result<()> {
  let filters: ResourceFilters = list.clone().into();
  let client = super::komodo_client().await?;

  let (tags, mut servers, mut stacks) = try_join!(
    client.read(ListTags::default()).map(|res| res.map(|res| res
      .into_iter()
      .map(|t| (t.id, t.name))
      .collect::<HashMap<_, _>>())),
    ServerListItem::list(client, &filters),
    StackListItem::list(client, &filters)
  )?;

  if !servers.is_empty() {
    fix_tags(&mut servers, &tags);
    print_items(servers, filters.format)?;
    println!();
  }

  if !stacks.is_empty() {
    fix_tags(&mut stacks, &tags);
    print_items(stacks, filters.format)?;
    println!();
  }

  Ok(())
}

async fn list_resources<T>(
  filters: &ResourceFilters,
) -> anyhow::Result<()>
where
  T: ListResources,
  ResourceListItem<T::Info>: PrintTable + Serialize,
{
  let client = crate::command::komodo_client().await?;
  let (mut resources, tags) = tokio::try_join!(
    T::list(client, filters),
    client.read(ListTags::default()).map(|res| res.map(|res| res
      .into_iter()
      .map(|t| (t.id, t.name))
      .collect::<HashMap<_, _>>()))
  )?;
  fix_tags(&mut resources, &tags);
  if !resources.is_empty() {
    print_items(resources, filters.format)?;
  }
  Ok(())
}

fn fix_tags<T>(
  resources: &mut Vec<ResourceListItem<T>>,
  tags: &HashMap<String, String>,
) {
  resources.iter_mut().for_each(|resource| {
    resource.tags.iter_mut().for_each(|id| {
      let Some(name) = tags.get(id) else {
        *id = String::new();
        return;
      };
      id.clone_from(name);
    });
  });
}

trait ListResources: Sized
where
  ResourceListItem<Self::Info>: PrintTable,
{
  type Info;
  async fn list(
    client: &KomodoClient,
    filters: &ResourceFilters,
  ) -> anyhow::Result<Vec<ResourceListItem<Self::Info>>>;
}

// LIST

impl ListResources for ServerListItem {
  type Info = ServerListItemInfo;
  async fn list(
    client: &KomodoClient,
    filters: &ResourceFilters,
  ) -> anyhow::Result<Vec<Self>> {
    let servers = client
      .read(ListServers {
        query: ResourceQuery::builder()
          .tags(filters.tags.clone())
          // .tag_behavior(TagQueryBehavior::Any)
          .build(),
      })
      .await?;
    let names = parse_wildcards(&filters.names);
    let server_wildcards = parse_wildcards(&filters.servers);
    let mut servers = servers
      .into_iter()
      .filter(|server| {
        let state_check = if filters.all {
          true
        } else if filters.down {
          !matches!(server.info.state, ServerState::Ok)
        } else {
          matches!(server.info.state, ServerState::Ok)
        };
        let name_items = &[server.name.as_str()];
        state_check
          && matches_wildcards(&names, name_items)
          && matches_wildcards(&server_wildcards, name_items)
      })
      .collect::<Vec<_>>();
    servers.sort_by(|a, b| {
      a.info.state.cmp(&b.info.state).then(a.name.cmp(&b.name))
    });
    Ok(servers)
  }
}

impl ListResources for StackListItem {
  type Info = StackListItemInfo;
  async fn list(
    client: &KomodoClient,
    filters: &ResourceFilters,
  ) -> anyhow::Result<Vec<Self>> {
    let (servers, mut stacks) = tokio::try_join!(
      client
        .read(ListServers {
          query: ResourceQuery::builder().build(),
        })
        .map(|res| res.map(|res| res
          .into_iter()
          .map(|s| (s.id.clone(), s))
          .collect::<HashMap<_, _>>())),
      client.read(ListStacks {
        query: ResourceQuery::builder()
          .tags(filters.tags.clone())
          // .tag_behavior(TagQueryBehavior::Any)
          .build(),
      })
    )?;
    stacks.iter_mut().for_each(|stack| {
      if stack.info.server_id.is_empty() {
        return;
      }
      let Some(server) = servers.get(&stack.info.server_id) else {
        return;
      };
      stack.info.server_id.clone_from(&server.name);
    });
    let names = parse_wildcards(&filters.names);
    let servers = parse_wildcards(&filters.servers);
    let mut stacks = stacks
      .into_iter()
      .filter(|stack| {
        let state_check = if filters.all {
          true
        } else if filters.down {
          !matches!(stack.info.state, StackState::Running)
        } else {
          matches!(stack.info.state, StackState::Running)
        };
        state_check
          && matches_wildcards(&names, &[stack.name.as_str()])
          && matches_wildcards(
            &servers,
            &[stack.info.server_id.as_str()],
          )
      })
      .collect::<Vec<_>>();
    stacks.sort_by(|a, b| {
      a.info.state.cmp(&b.info.state).then(
        a.name
          .cmp(&b.name)
          .then(a.info.server_id.cmp(&b.info.server_id)),
      )
    });
    Ok(stacks)
  }
}

// TABLE

impl PrintTable for ResourceListItem<ServerListItemInfo> {
  fn header() -> &'static [&'static str] {
    &["Server", "State", "Address", "Tags"]
  }
  fn row(self) -> Vec<Cell> {
    let color = match self.info.state {
      ServerState::Ok => Color::Green,
      ServerState::NotOk => Color::Red,
      ServerState::Disabled => Color::Blue,
    };
    vec![
      Cell::new(self.name).add_attribute(Attribute::Bold),
      Cell::new(self.info.state.to_string())
        .fg(color)
        .add_attribute(Attribute::Bold),
      Cell::new(self.info.address),
      Cell::new(self.tags.join(", ")),
    ]
  }
}

impl PrintTable for ResourceListItem<StackListItemInfo> {
  fn header() -> &'static [&'static str] {
    &["Stack", "State", "Server", "Tags"]
  }
  fn row(self) -> Vec<comfy_table::Cell> {
    let color = match self.info.state {
      StackState::Down => Color::Blue,
      StackState::Running => Color::Green,
      StackState::Paused => Color::DarkYellow,
      StackState::Unknown => Color::Magenta,
      _ => Color::Red,
    };
    vec![
      Cell::new(self.name).add_attribute(Attribute::Bold),
      Cell::new(self.info.state.to_string())
        .fg(color)
        .add_attribute(Attribute::Bold),
      Cell::new(self.info.server_id),
      Cell::new(self.tags.join(", ")),
    ]
  }
}
