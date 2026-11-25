use std::path::Path;

use pyo3::prelude::*;
use regex::Regex;

use crate::{Result, StateIterator, python::macros::repr::pyrepr};

#[pymethods]
impl StateIterator {
    /// State directory whose names this iterator lists
    #[getter]
    fn state_dir(&self) -> &Path {
        &self.registry.state_dir
    }

    /// Regular expression names must match, or `None` when every name is listed
    #[getter]
    fn pattern(&self) -> Option<&str> {
        self.pattern.as_ref().map(Regex::as_str)
    }

    /// Returns the iterator itself
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Returns the next saved state name, in no particular order
    ///
    /// Raises `OSError` when part of the state directory cannot be read
    fn __next__(&mut self) -> Result<Option<String>> {
        Iterator::next(self).transpose()
    }

    /// Returns the class name with the state directory and the pattern
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, state_dir, pattern)
    }
}
