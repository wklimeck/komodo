use std::path::Path;

use command::run_komodo_command;
use formatting::format_serror;
use komodo_client::entities::{
  all_logs_success, update::Log, CloneArgs,
};

pub async fn init_folder_as_repo(
  folder_path: &Path,
  args: &CloneArgs,
  access_token: Option<&str>,
  logs: &mut Vec<Log>,
) {
  // let folder_path = args.path(repo_dir);
  // Initialize the folder as a git repo
  let init_repo = run_komodo_command(
    "Git Init",
    folder_path.as_ref(),
    "git init",
    false,
  )
  .await;
  logs.push(init_repo);
  if !all_logs_success(&logs) {
    return;
  }

  let repo_url = match args.remote_url(access_token.as_deref()) {
    Ok(url) => url,
    Err(e) => {
      logs
        .push(Log::error("Add git remote", format_serror(&e.into())));
      return;
    }
  };

  // Set remote url
  let mut set_remote = run_komodo_command(
    "Add git remote",
    folder_path.as_ref(),
    format!("git remote add origin {repo_url}"),
    false,
  )
  .await;
  // Sanitize the output
  if let Some(token) = &access_token {
    set_remote.command = set_remote.command.replace(token, "<TOKEN>");
    set_remote.stdout = set_remote.stdout.replace(token, "<TOKEN>");
    set_remote.stderr = set_remote.stderr.replace(token, "<TOKEN>");
  }
  if !set_remote.success {
    logs.push(set_remote);
    return;
  }

  // Set branch.
  let init_repo = run_komodo_command(
    "Set Branch",
    folder_path.as_ref(),
    format!("git switch -c {}", args.branch),
    false,
  )
  .await;
  if !init_repo.success {
    logs.push(init_repo);
    return;
  }
}
