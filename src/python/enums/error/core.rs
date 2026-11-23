use pyo3::{
    PyErr,
    exceptions::{PyKeyError, PyValueError},
};

use crate::{
    Error, Mismatch,
    python::exceptions::{
        StateConflictError, StateCorruptError, StateError, StateMismatchError, StateOutdatedError,
    },
};

impl From<Error> for PyErr {
    fn from(error: Error) -> Self {
        let message = error.to_string();

        match error {
            Error::Io(io) => io.into(),
            Error::NotFound(name) => PyKeyError::new_err(name),
            Error::Alias { name } => Self::new::<StateError, _>((message, name)),
            Error::Corrupt { name, .. } => Self::new::<StateCorruptError, _>((message, name)),
            Error::Mismatch { name, kind } => match kind {
                Mismatch::Path | Mismatch::Size | Mismatch::Mtime | Mismatch::Missing => {
                    Self::new::<StateOutdatedError, _>((message, name))
                }
                Mismatch::Checksum | Mismatch::Name { .. } => {
                    Self::new::<StateCorruptError, _>((message, name))
                }
                Mismatch::Stale => Self::new::<StateConflictError, _>((message, name)),
                Mismatch::Identity | Mismatch::Unsealed => {
                    Self::new::<StateMismatchError, _>((message, name))
                }
            },
            Error::Name(_)
            | Error::InvalidRange
            | Error::InvalidMtime
            | Error::Regex(_)
            | Error::Serde(_)
            | Error::Utf8(_) => PyValueError::new_err(message),
        }
    }
}
