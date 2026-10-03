use pyo3::{
    PyErr,
    exceptions::{PyKeyError, PyValueError},
};

use crate::{Error, python::exceptions::StateError};

impl From<Error> for PyErr {
    fn from(error: Error) -> Self {
        match error {
            Error::Io(error) => error.into(),
            Error::NotFound(name) => PyKeyError::new_err(name),
            error @ (Error::InvalidRange { .. } | Error::Utf8(_)) => {
                PyValueError::new_err(error.to_string())
            }
            error => Self::new::<StateError, _>(error.to_string()),
        }
    }
}
