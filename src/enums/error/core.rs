use std::path::PathBuf;

use crate::enums::error::{mismatch::Mismatch, path::PathError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Regex(#[from] regex::Error),
    #[error(transparent)]
    Time(#[from] std::time::SystemTimeError),
    #[error(transparent)]
    Path(#[from] PathError),
    #[error(transparent)]
    Mismatch(#[from] Mismatch),
    #[error("state not found: {0}")]
    Missing(String),
    #[error("start ({start}) must be <= end ({end})")]
    Bounds { start: u64, end: u64 },
    #[error("not a file: {}", .0.display())]
    NotAFile(PathBuf),
    #[error("invalid mtime: {0}")]
    Mtime(i64),
}
