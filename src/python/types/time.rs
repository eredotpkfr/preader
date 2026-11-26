use pyo3::prelude::*;

use crate::{Timestamps, python::macros::repr::pyrepr};

#[pymethods]
impl Timestamps {
    /// Returns the class name with both timestamps
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, created_at, updated_at)
    }
}
