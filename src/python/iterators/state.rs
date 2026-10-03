use pyo3::{exceptions::PyStopIteration, prelude::*};

use crate::{
    StateIterator,
    python::{macros::repr::pyrepr, utils::text::quote},
};

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

    fn __repr__(&self) -> String {
        pyrepr!("StateIterator" {
            state_dir = quote(self.state_dir.display()),
            pattern = self.pattern.as_ref().map_or_else(|| "None".to_owned(), quote),
        })
    }
}
