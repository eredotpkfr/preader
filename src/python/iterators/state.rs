use std::path::Path;

use pyo3::{exceptions::PyStopIteration, prelude::*};
use regex::Regex;

use crate::{StateIterator, python::macros::repr::pyrepr};

#[pymethods]
impl StateIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<String> {
        let Some(name) = Iterator::next(self) else {
            return Err(PyStopIteration::new_err(()));
        };

        Ok(name?)
    }

    #[getter]
    fn state_dir(&self) -> &Path {
        &self.manager.state_dir
    }

    #[getter]
    fn pattern(&self) -> Option<&str> {
        self.pattern.as_ref().map(Regex::as_str)
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, state_dir, pattern)
    }
}
