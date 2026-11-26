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

/// Converts a crate error into the Python exception it raises
///
/// [`Error::Io`] becomes the matching `OSError` subclass and [`Error::NotFound`] becomes
/// `KeyError`. [`Error::Alias`], [`Error::Corrupt`] and every [`Mismatch`] kind become `StateError`
/// or one of its subclasses carrying the state name, and the remaining variants become `ValueError`
impl From<Error> for PyErr {
    /// Builds the exception from the error's message, adding `name` for state errors
    ///
    /// `KeyError` carries only the name
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
