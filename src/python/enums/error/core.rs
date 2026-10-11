use pyo3::{
    PyErr,
    exceptions::{PyKeyError, PyValueError},
};

use crate::{
    Error, NameError,
    python::exceptions::{StateError, StateMismatchError},
};

impl From<Error> for PyErr {
    fn from(error: Error) -> Self {
        let message = error.to_string();

        match error {
            Error::Io(error) => error.into(),
            Error::NotFound(name) => PyKeyError::new_err(name),
            Error::Mismatch(_) => Self::new::<StateMismatchError, _>(message),
            Error::Name(NameError::Alias) | Error::Corrupt { .. } => {
                Self::new::<StateError, _>(message)
            }
            Error::Name(_)
            | Error::InvalidRange
            | Error::InvalidMtime
            | Error::Regex(_)
            | Error::Serde(_)
            | Error::Utf8(_) => PyValueError::new_err(message),
        }
    }
}
