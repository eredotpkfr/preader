use crate::enums::error::{mismatch::Mismatch, name::NameError};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Regex(#[from] regex::Error),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    Name(#[from] NameError),
    #[error(transparent)]
    Mismatch(#[from] Mismatch),
    #[error("state '{name}' is corrupt: {error}")]
    Corrupt {
        name: String,
        error: serde_json::Error,
    },
    #[error("state '{0}' not found")]
    NotFound(String),
    #[error("start must not be greater than end")]
    InvalidRange,
    #[error("file mtime is out of range")]
    InvalidMtime,
}

impl From<walkdir::Error> for Error {
    fn from(error: walkdir::Error) -> Self {
        Self::Io(error.into())
    }
}
