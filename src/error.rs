use std::path::{Path, PathBuf};

use crate::diagnostic::Location;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read `{path}`: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to write `{path}`: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("invalid config {}: {message}", position(.path, *.location))]
    Config {
        path: PathBuf,
        location: Option<Location>,
        message: String,
    },

    #[error("path `{0}` does not exist")]
    PathNotFound(PathBuf),

    #[error("invalid glob pattern `{pattern}`: {message}")]
    Glob { pattern: String, message: String },

    #[error("unknown ODCS version `{value}`; supported versions: {supported}")]
    UnknownVersion { value: String, supported: String },

    #[error("no contract files found (pass --allow-empty to accept this)")]
    NoFiles,

    #[error("embedded ODCS schema {version} could not be compiled: {message}")]
    Schema {
        version: &'static str,
        message: String,
    },

    #[error("`{path}` already exists (pass --force to overwrite)")]
    AlreadyExists { path: PathBuf },

    #[error("failed to write output: {0}")]
    Output(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

fn position(path: &Path, location: Option<Location>) -> String {
    location.map_or_else(
        || path.display().to_string(),
        |location| format!("{}:{}:{}", path.display(), location.line, location.column),
    )
}
