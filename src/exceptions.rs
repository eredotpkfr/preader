use pyo3::{PyErr, create_exception, exceptions::PyException};

create_exception!(preader, PReaderConfigError, PyException);

impl PReaderConfigError {
    pub fn from_anyhow(error: anyhow::Error) -> PyErr {
        Self::new_err(format!("{error:#}"))
    }
}
