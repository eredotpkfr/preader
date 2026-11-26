use pyo3::prelude::*;

use crate::{FileMetadata, python::macros::repr::pyrepr};

#[pymethods]
impl FileMetadata {
    /// Returns the class name with the path, size, mtime and fingerprint
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, path, size, mtime, fingerprint)
    }
}
