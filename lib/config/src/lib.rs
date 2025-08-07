use std::{
  collections::HashSet,
  fs::File,
  io::Read,
  path::{Path, PathBuf},
};

use colored::Colorize;
use indexmap::IndexSet;
use serde::de::DeserializeOwned;

mod error;

pub use error::Error;

pub type Result<T> = ::core::result::Result<T, Error>;

/// parse paths that are either directories or files
pub fn parse_config_paths<T: DeserializeOwned>(
  paths: &[&Path],
  match_wildcards: &[&str],
  ignore_file_name: &str,
  merge_nested: bool,
  extend_array: bool,
  debug_print: bool,
) -> Result<T> {
  let mut wildcards = Vec::with_capacity(match_wildcards.len());
  for &wc in match_wildcards {
    match wildcard::Wildcard::new(wc.as_bytes()) {
      Ok(wc) => wildcards.push(wc),
      Err(e) => {
        eprintln!(
          "{}: Keyword '{}' is invalid wildcard | {e:?}",
          "ERROR".red(),
          wc.bold(),
        );
      }
    }
  }
  let mut all_files = IndexSet::new();
  for &path in paths {
    let Ok(metadata) = std::fs::metadata(path) else {
      continue;
    };
    if metadata.is_dir() {
      // Collect ignore paths
      let mut ignores = HashSet::new();
      add_ignores(path, ignore_file_name, &mut ignores);

      if debug_print && !ignores.is_empty() {
        println!(
          "{}: {}: {ignores:?}",
          "DEBUG".cyan(),
          format!(
            "{} {path:?} {}",
            "Config Path".dimmed(),
            "Ignores".dimmed()
          ),
        );
      }

      let mut files = HashSet::new();
      add_files(&mut files, path, &wildcards, &ignores);
      let mut files = files.into_iter().collect::<Vec<_>>();
      files.sort();
      all_files.extend(files);
    } else if metadata.is_file() {
      all_files.insert(path.to_path_buf());
    }
  }
  println!(
    "{}: {}: {all_files:?}",
    "INFO".green(),
    "Found Files".dimmed()
  );
  parse_config_files(
    &all_files.into_iter().collect::<Vec<_>>(),
    merge_nested,
    extend_array,
  )
}

fn ignore_dir(path: &Path, ignores: &HashSet<PathBuf>) -> bool {
  const IGNORE: &[&str] = &["target", "node_modules", ".git"];
  IGNORE.iter().any(|ignore| path.ends_with(ignore))
    || ignores.contains(path)
}

fn add_files(
  files: &mut HashSet<PathBuf>,
  folder: &Path,
  wildcards: &[wildcard::Wildcard],
  ignores: &HashSet<PathBuf>,
) {
  let Ok(folder) = folder.canonicalize() else {
    return;
  };

  if ignores.contains(&folder) {
    return;
  }

  let Ok(read_dir) = std::fs::read_dir(folder) else {
    return;
  };
  for dir_entry in read_dir.flatten() {
    let path = dir_entry.path();
    if ignore_dir(&path, ignores) {
      continue;
    }
    let Ok(metadata) = dir_entry.metadata() else {
      continue;
    };
    if metadata.is_file() {
      // BASE CASE
      let file_name = dir_entry.file_name();
      let Some(file_name) = file_name.to_str() else {
        continue;
      };
      // Ensure file name matches a wildcard keyword
      if !wildcards.is_empty()
        && !wildcards
          .iter()
          .any(|wc| wc.is_match(file_name.as_bytes()))
      {
        continue;
      }
      let Ok(path) = path.canonicalize() else {
        continue;
      };
      files.insert(path);
    } else if metadata.is_dir() {
      // RECURSIVE CASE
      add_files(files, &dir_entry.path(), wildcards, ignores);
    }
  }
}

fn add_ignores(
  folder: &Path,
  ignore_file_name: &str,
  ignores: &mut HashSet<PathBuf>,
) {
  let Ok(folder) = folder.canonicalize() else {
    return;
  };

  if ignores.contains(&folder) {
    return;
  }

  // Add any ignores in this folder
  if let Ok(ignore) =
    std::fs::read_to_string(folder.join(ignore_file_name))
  {
    ignores.extend(
      ignore
        .split('\n')
        .map(|line| line.trim())
        // Ignore empty / commented out lines
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        // Remove end of line comments
        .map(|line| {
          line.split_once('#').map(|res| res.0.trim()).unwrap_or(line)
        })
        .flat_map(|line| folder.join(line).canonicalize()),
    );
  };

  if ignores.contains(&folder) {
    return;
  }

  // Then check any sub directories
  let Ok(entries) = std::fs::read_dir(folder) else {
    return;
  };
  for entry in entries.flatten() {
    let Ok(path) = entry.path().canonicalize() else {
      continue;
    };
    if ignore_dir(&path, ignores) {
      continue;
    }
    let Ok(metadata) = entry.metadata() else {
      continue;
    };
    if !metadata.is_dir() {
      continue;
    }
    add_ignores(&path, ignore_file_name, ignores);
  }
}

/// parses multiple config files
pub fn parse_config_files<T: DeserializeOwned>(
  files: &[PathBuf],
  merge_nested: bool,
  extend_array: bool,
) -> Result<T> {
  let mut target = serde_json::Map::new();

  for file in files {
    let source = match parse_config_file(file) {
      Ok(source) => source,
      Err(e) => {
        eprintln!(
          "{}: Failed to parse config at {file:?} | {e}",
          "WARN".yellow()
        );
        continue;
      }
    };
    target = match merge_objects(
      target.clone(),
      source,
      merge_nested,
      extend_array,
    ) {
      Ok(target) => target,
      Err(e) => {
        eprint!(
          "{}: Failed to merge config at {file:?} | {e}",
          "WARN".yellow()
        );
        target
      }
    };
  }

  serde_json::from_str(
    &serde_json::to_string(&target)
      .map_err(|e| Error::SerializeFinalJson { e })?,
  )
  .map_err(|e| Error::ParseFinalJson { e })
}

/// parses a single config file
pub fn parse_config_file<T: DeserializeOwned>(
  file: &Path,
) -> Result<T> {
  let mut file_handle =
    File::open(file).map_err(|e| Error::FileOpen {
      e,
      path: file.to_path_buf(),
    })?;
  let config = match file.extension().and_then(|e| e.to_str()) {
    Some("toml") => {
      let mut contents = String::new();
      file_handle.read_to_string(&mut contents).map_err(|e| {
        Error::ReadFileContents {
          e,
          path: file.to_path_buf(),
        }
      })?;
      toml::from_str(&contents).map_err(|e| Error::ParseToml {
        e,
        path: file.to_path_buf(),
      })?
    }
    Some("json") => {
      serde_json::from_reader(file_handle).map_err(|e| {
        Error::ParseJson {
          e,
          path: file.to_path_buf(),
        }
      })?
    }
    Some(_) | None => {
      return Err(Error::UnsupportedFileType {
        path: file.to_path_buf(),
      });
    }
  };
  Ok(config)
}

/// object is serde_json::Map<String, serde_json::Value>
/// source will overide target
/// will recurse when field is object if merge_object = true, otherwise object will be replaced
/// will extend when field is array if extend_array = true, otherwise array will be replaced
/// will return error when types on source and target fields do not match
fn merge_objects(
  mut target: serde_json::Map<String, serde_json::Value>,
  source: serde_json::Map<String, serde_json::Value>,
  merge_nested: bool,
  extend_array: bool,
) -> Result<serde_json::Map<String, serde_json::Value>> {
  for (key, value) in source {
    let Some(curr) = target.remove(&key) else {
      target.insert(key, value);
      continue;
    };
    match curr {
      serde_json::Value::Object(target_obj) => {
        if !merge_nested {
          target.insert(key, value);
          continue;
        }
        match value {
          serde_json::Value::Object(source_obj) => {
            target.insert(
              key,
              serde_json::Value::Object(merge_objects(
                target_obj,
                source_obj,
                merge_nested,
                extend_array,
              )?),
            );
          }
          _ => {
            return Err(Error::ObjectFieldTypeMismatch {
              key,
              value: Box::new(value),
            });
          }
        }
      }
      serde_json::Value::Array(mut target_arr) => {
        if !extend_array {
          target.insert(key, value);
          continue;
        }
        match value {
          serde_json::Value::Array(source_arr) => {
            target_arr.extend(source_arr);
            target.insert(key, serde_json::Value::Array(target_arr));
          }
          _ => {
            return Err(Error::ArrayFieldTypeMismatch {
              key,
              value: Box::new(value),
            });
          }
        }
      }
      _ => {
        target.insert(key, value);
      }
    }
  }
  Ok(target)
}
