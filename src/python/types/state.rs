use pyo3::prelude::*;

use crate::{FileMetadata, State, Timestamps, python::macros::repr::pyrepr};

#[pymethods]
impl State {
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    #[getter]
    fn file(&self) -> FileMetadata {
        self.file.clone()
    }

    #[getter]
    fn position(&self) -> u64 {
        self.position
    }

    #[getter]
    fn timestamps(&self) -> Timestamps {
        self.timestamps.clone()
    }

    #[pyo3(name = "percent")]
    fn py_percent(&self) -> f64 {
        self.percent()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, name, file, position, timestamps)
    }
}
