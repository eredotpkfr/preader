use pyo3::prelude::*;

use crate::{FileMetadata, State, Timestamps, python::macros::repr::pyrepr};

#[pymethods]
impl State {
    /// Name the state is saved under
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    /// Metadata of the tracked file, recorded when the state was created or last resynced
    #[getter]
    fn file(&self) -> FileMetadata {
        self.file.clone()
    }

    /// Byte offset just past the last finished item, where a resumed iterator starts
    #[getter]
    fn position(&self) -> u64 {
        self.position
    }

    /// Times the state was created and last saved or resynced
    #[getter]
    fn timestamps(&self) -> Timestamps {
        self.timestamps.clone()
    }

    /// Returns `position` as a share of the recorded file size, from 0.0 to 100.0
    ///
    /// It returns 0.0 for an empty file
    #[pyo3(name = "percent")]
    fn py_percent(&self) -> f64 {
        self.percent()
    }

    /// Returns the class name with the name, file, position and timestamps
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, name, file, position, timestamps)
    }
}
