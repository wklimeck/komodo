use std::{
  fs::File,
  io::Read,
  path::{Path, PathBuf},
};

use colored::Colorize;
use serde::de::DeserializeOwned;

mod error;

pub use error::Error;

pub type Result<T> = ::core::result::Result<T, Error>;

/// parse paths that are either directories or files
pub fn parse_config_paths<T: DeserializeOwned>(
  paths: &[&Path],
  match_wildcards: &[&str],
  merge_nested: bool,
  extend_array: bool,
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
  let mut files = Vec::with_capacity(paths.len());
  for &path in paths {
    let Ok(metadata) = std::fs::metadata(path) else {
      continue;
    };
    if metadata.is_dir() {
      extend_with_file_names_in_dir(&mut files, path, &wildcards);
    } else if metadata.is_file() {
      files.push(path.to_path_buf());
    }
  }
  files.sort();
  println!("{}: Found files: {files:?}", "INFO".green());
  parse_config_files(&files, merge_nested, extend_array)
}

fn ignore_dir(path: &Path) -> bool {
  const IGNORE: &[&str] = &["target", "node_modules", ".git"];
  IGNORE.iter().any(|ignore| path.ends_with(ignore))
}

/// will sort file names alphabetically
fn extend_with_file_names_in_dir(
  files: &mut Vec<PathBuf>,
  dir_path: &Path,
  wildcards: &[wildcard::Wildcard],
) {
  let Ok(read_dir) = std::fs::read_dir(dir_path) else {
    return;
  };
  for dir_entry in read_dir {
    let Ok(dir_entry) = dir_entry else {
      continue;
    };
    let path = dir_entry.path();
    if ignore_dir(&path) {
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
      if wildcards.is_empty()
        || wildcards
          .iter()
          .any(|wc| wc.is_match(file_name.as_bytes()))
      {
        files.push(path);
      }
    } else if metadata.is_dir() {
      extend_with_file_names_in_dir(
        files,
        &dir_entry.path(),
        wildcards,
      );
    }
  }
}

/// parses multiple config files
pub fn parse_config_files<T: DeserializeOwned>(
  paths: &[PathBuf],
  merge_nested: bool,
  extend_array: bool,
) -> Result<T> {
  let mut target = serde_json::Map::new();

  for path in paths {
    let source = match parse_config_file(path) {
      Ok(source) => source,
      Err(e) => {
        eprintln!(
          "{}: Failed to parse config at {path:?} | {e}",
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
          "{}: Failed to merge config at {path:?} | {e}",
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
  path: &Path,
) -> Result<T> {
  let mut file = File::open(path).map_err(|e| Error::FileOpen {
    e,
    path: path.to_path_buf(),
  })?;
  let config = match path.extension().and_then(|e| e.to_str()) {
    Some("toml") => {
      let mut contents = String::new();
      file.read_to_string(&mut contents).map_err(|e| {
        Error::ReadFileContents {
          e,
          path: path.to_path_buf(),
        }
      })?;
      toml::from_str(&contents).map_err(|e| Error::ParseToml {
        e,
        path: path.to_path_buf(),
      })?
    }
    Some("json") => {
      serde_json::from_reader(file).map_err(|e| Error::ParseJson {
        e,
        path: path.to_path_buf(),
      })?
    }
    Some(_) | None => {
      return Err(Error::UnsupportedFileType {
        path: path.to_path_buf(),
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
