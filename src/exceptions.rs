use pyo3::{PyErr, create_exception, exceptions::PyException};

create_exception!(preader, PReaderStateError, PyException);

impl PReaderStateError {
    pub fn from_anyhow(error: anyhow::Error) -> PyErr {
        Self::new_err(format!("{error:#}"))
    }
}
