use pyo3::{PyErr, create_exception, exceptions::PyException};

create_exception!(preader, StateError, PyException);

impl StateError {
    pub fn from_io(error: std::io::Error) -> PyErr {
        Self::new_err(format!("io failed ({:?}): {error:#}", error.kind()))
    }

    pub fn from_anyhow(error: anyhow::Error) -> PyErr {
        Self::new_err(format!("{error:#}"))
    }

    pub fn from_serde(error: serde_json::Error) -> PyErr {
        Self::new_err(format!("serde failed: {error:#}"))
    }

    pub fn from_regex(error: regex::Error) -> PyErr {
        Self::new_err(format!("regex failed: {error:#}"))
    }
}
