use std::io::{self, ErrorKind};

use crate::enums::error::{mismatch::Mismatch, name::NameError};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Regex(#[from] regex::Error),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    Name(#[from] NameError),
    #[error("state '{name}': {kind}")]
    Mismatch { name: String, kind: Mismatch },
    #[error("state '{name}': state file is not valid ({error})")]
    Corrupt {
        name: String,
        error: serde_json::Error,
    },
    #[error("state '{name}': name is an alias of another entry (symlink, case or Unicode)")]
    Alias { name: String },
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

impl Error {
    pub(crate) fn alias(name: &str) -> Self {
        Self::Alias {
            name: name.to_owned(),
        }
    }

    pub(crate) fn mismatch(name: &str, kind: Mismatch) -> Self {
        Self::Mismatch {
            name: name.to_owned(),
            kind,
        }
    }

    pub(crate) fn corrupt(name: &str) -> impl FnOnce(serde_json::Error) -> Self {
        move |error| Self::Corrupt {
            name: name.to_owned(),
            error,
        }
    }

    pub(crate) fn not_found(name: &str) -> impl FnOnce(io::Error) -> Self {
        move |error| match error.kind() {
            ErrorKind::NotFound => Self::NotFound(name.to_owned()),
            _ => error.into(),
        }
    }

    pub(crate) fn unsealed(error: Self) -> Self {
        match error {
            Self::Mismatch {
                name,
                kind: Mismatch::Checksum,
            } => Self::Mismatch {
                name,
                kind: Mismatch::Unsealed,
            },
            error => error,
        }
    }
}
