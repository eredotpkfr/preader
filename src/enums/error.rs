use pyo3::{PyErr, exceptions::PyKeyError};
use thiserror::Error as ThisError;

use crate::StateError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("io failed ({:?}): {}", .0.kind(), .0)]
    Io(#[from] std::io::Error),
    #[error("serde failed: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("regex failed: {0}")]
    Regex(#[from] regex::Error),
    #[error("{0}")]
    Missing(String),
    #[error("{0:#}")]
    Message(#[from] anyhow::Error),
}

impl From<Error> for PyErr {
    fn from(error: Error) -> Self {
        match error {
            Error::Missing(name) => PyKeyError::new_err(name),
            error => StateError::new_err(error.to_string()),
        }
    }
}
