use std::path::PathBuf;

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

    #[error("invalid config `{path}`: {message}")]
    Config { path: PathBuf, message: String },

    #[error("path `{0}` does not exist")]
    PathNotFound(PathBuf),

    #[error("invalid glob pattern `{pattern}`: {message}")]
    Glob { pattern: String, message: String },

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
