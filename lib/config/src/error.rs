use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(
    "types on field {key} do not match | got {value:?}, expected object"
  )]
  ObjectFieldTypeMismatch {
    key: String,
    value: Box<dyn std::fmt::Debug>,
  },

  #[error(
    "types on field {key} do not match | got {value:?}, expected array"
  )]
  ArrayFieldTypeMismatch {
    key: String,
    value: Box<dyn std::fmt::Debug>,
  },

  #[error("failed to open file at {path} | {e:?}")]
  FileOpen { e: std::io::Error, path: PathBuf },

  #[error("failed to read contents of file at {path} | {e:?}")]
  ReadFileContents { e: std::io::Error, path: PathBuf },

  #[error("failed to parse toml file at {path} | {e:?}")]
  ParseToml { e: toml::de::Error, path: PathBuf },

  #[error("failed to parse json file at {path} | {e:?}")]
  ParseJson { e: serde_json::Error, path: PathBuf },

  #[error("unsupported file type at {path}")]
  UnsupportedFileType { path: PathBuf },

  #[error("failed to parse merged config into final type | {e:?}")]
  ParseFinalJson { e: serde_json::Error },

  #[error("failed to serialize merged config to string | {e:?}")]
  SerializeFinalJson { e: serde_json::Error },

  #[error("failed to read directory at {path:?}")]
  ReadDir { path: PathBuf, e: std::io::Error },

  #[error("failed to get file handle for file in directory {path:?}")]
  DirFile { e: std::io::Error, path: PathBuf },

  #[error("failed to get file name for file at {path:?}")]
  GetFileName { path: PathBuf },

  #[error("failed to get metadata for path {path:?} | {e:?}")]
  ReadPathMetaData { path: PathBuf, e: std::io::Error },

  #[error("parsed value is not object")]
  ValueIsNotObject,
}
