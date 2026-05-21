use pyo3::{prelude::*, types::PyAny};

use crate::types::progress::ProgressState;

#[pyclass]
pub struct PReaderItem {
    #[pyo3(get)]
    pub value: Py<PyAny>,
    #[pyo3(get)]
    pub bytes_read: u64,
    #[pyo3(get)]
    pub total_bytes: u64,
}

impl<T> From<(&mut ProgressState, Py<T>)> for PReaderItem {
    fn from(tuple: (&mut ProgressState, Py<T>)) -> Self {
        Self {
            value: tuple.1.into_any(),
            bytes_read: tuple.0.bytes_read,
            total_bytes: tuple.0.total_bytes,
        }
    }
}

#[pymethods]
impl PReaderItem {
    #[getter]
    fn percentage(&self) -> f64 {
        self.bytes_read as f64 * 100.0 / self.total_bytes.max(1) as f64
    }
}
