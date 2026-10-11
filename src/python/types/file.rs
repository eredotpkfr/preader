use pyo3::prelude::*;

use crate::{FileMetadata, python::macros::repr::pyrepr};

#[pymethods]
impl FileMetadata {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, path, size, mtime, fingerprint)
    }
}
