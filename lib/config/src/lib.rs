//! # Komodo Config
//!
//! This library is used to parse Core, Periphery, and CLI config files.
//! It supports interpolating in environment variables (only '${VAR}' syntax),
//! as well as merging together multiple files into a final configuration object.

use std::{
  collections::HashSet,
  fs::File,
  io::Read,
  path::{Path, PathBuf},
};

use colored::Colorize;
use indexmap::IndexSet;
use serde::{Serialize, de::DeserializeOwned};

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
          format_args!(
            "{} {path:?} {}",
            "Config Path".dimmed(),
            "Ignores".dimmed()
          ),
        );
      }

      let mut files = Vec::new();
      add_files(&mut files, path, &wildcards, &ignores);
      files.sort_by(|(a_index, a_path), (b_index, b_path)| {
        match a_index.cmp(b_index) {
          std::cmp::Ordering::Less => {
            return std::cmp::Ordering::Less;
          }
          std::cmp::Ordering::Greater => {
            return std::cmp::Ordering::Greater;
          }
          std::cmp::Ordering::Equal => {}
        }
        a_path.cmp(b_path)
      });
      all_files.extend(files.into_iter().map(|(_, path)| path));
    } else if metadata.is_file() {
      let path = path.to_path_buf();
      // If the same path comes up again later on, it should be removed and
      // reinserted so it maintains higher priority.
      all_files.shift_remove(&path);
      all_files.insert(path);
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
  // stores index of matching keyword as well as path
  files: &mut Vec<(usize, PathBuf)>,
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
      let index = if wildcards.is_empty() {
        0
      } else if let Some(index) = wildcards
        .iter()
        .position(|wc| wc.is_match(file_name.as_bytes()))
      {
        index
      } else {
        continue;
      };
      let Ok(path) = path.canonicalize() else {
        continue;
      };
      files.push((index, path));
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
        eprintln!("{}: {e}", "WARN".yellow());
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
        eprint!("{}: {e}", "WARN".yellow());
        target
      }
    };
  }

  serde_json::from_value(serde_json::Value::Object(target))
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
  let mut contents = String::new();
  file_handle.read_to_string(&mut contents).map_err(|e| {
    Error::ReadFileContents {
      e,
      path: file.to_path_buf(),
    }
  })?;
  let contents = interpolate_env(&contents);
  let config = match file.extension().and_then(|e| e.to_str()) {
    Some("toml") => {
      toml::from_str(&contents).map_err(|e| Error::ParseToml {
        e,
        path: file.to_path_buf(),
      })?
    }
    Some("yaml") | Some("yml") => serde_yaml_ng::from_str(&contents)
      .map_err(|e| Error::ParseYaml {
        e,
        path: file.to_path_buf(),
      })?,
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

/// - Object is serde_json::Map<String, serde_json::Value>.
/// - Source will overide target.
/// - Will recurse when field is object if merge_object = true, otherwise object will be replaced.
/// - Will extend when field is array if extend_array = true, otherwise array will be replaced.
/// - Will return error when types on source and target fields do not match.
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

/// Source will overide target
pub fn merge_config<T: Serialize + DeserializeOwned>(
  target: T,
  source: T,
  merge_nested: bool,
  extend_array: bool,
) -> Result<T> {
  let serde_json::Value::Object(target) =
    serde_json::to_value(target)
      .map_err(|e| Error::SerializeJson { e })?
  else {
    return Err(Error::ValueIsNotObject);
  };
  let serde_json::Value::Object(source) =
    serde_json::to_value(source)
      .map_err(|e| Error::SerializeJson { e })?
  else {
    return Err(Error::ValueIsNotObject);
  };
  let object =
    merge_objects(target, source, merge_nested, extend_array)?;
  serde_json::from_value(serde_json::Value::Object(object))
    .map_err(|e| Error::ParseFinalJson { e })
}

/// Only supports '${VAR}' syntax
fn interpolate_env(input: &str) -> String {
  let re = regex::Regex::new(r"\$\{([A-Za-z0-9_]+)\}").unwrap();
  let first_pass = re
    .replace_all(input, |caps: &regex::Captures| {
      let var_name = &caps[1];
      std::env::var(var_name).unwrap_or_default()
    })
    .into_owned();
  // Do it twice in case any env vars expand again to env vars
  re.replace_all(&first_pass, |caps: &regex::Captures| {
    let var_name = &caps[1];
    std::env::var(var_name).unwrap_or_default()
  })
  .into_owned()
}
