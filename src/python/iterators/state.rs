use std::path::Path;

use pyo3::prelude::*;
use regex::Regex;

use crate::{Result, StateIterator, python::macros::repr::pyrepr};

#[pymethods]
impl StateIterator {
    #[getter]
    fn state_dir(&self) -> &Path {
        &self.registry.state_dir
    }

    #[getter]
    fn pattern(&self) -> Option<&str> {
        self.pattern.as_ref().map(Regex::as_str)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Result<Option<String>> {
        Iterator::next(self).transpose()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, state_dir, pattern)
    }
}
