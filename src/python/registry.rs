use std::path::{Path, PathBuf};

use pyo3::prelude::*;

use crate::{
    State, StateIterator, StateRegistry,
    python::{macros::repr::pyrepr, utils::text::quote},
};

#[pymethods]
impl StateRegistry {
    #[getter]
    #[pyo3(name = "state_dir")]
    fn py_state_dir(&self) -> &Path {
        self.state_dir()
    }

    #[pyo3(name = "names")]
    fn py_names(&self) -> PyResult<StateIterator> {
        Ok(self.names()?)
    }

    #[pyo3(name = "search")]
    fn py_search(&self, pattern: &str) -> PyResult<StateIterator> {
        Ok(self.search(pattern)?)
    }

    #[pyo3(name = "load")]
    fn py_load(&self, name: &str) -> PyResult<State> {
        Ok(self.load(name)?)
    }

    #[pyo3(name = "find")]
    fn py_find(&self, name: &str) -> Option<State> {
        self.find(name)
    }

    #[pyo3(name = "all")]
    fn py_all(&self) -> PyResult<Vec<State>> {
        Ok(self.all()?)
    }

    #[pyo3(name = "exists")]
    fn py_exists(&self, name: &str) -> bool {
        self.exists(name)
    }

    #[pyo3(name = "path")]
    fn py_path(&self, name: &str) -> PyResult<PathBuf> {
        Ok(self.path(name)?)
    }

    #[pyo3(name = "delete")]
    fn py_delete(&self, name: &str) -> PyResult<()> {
        Ok(self.delete(name)?)
    }

    #[pyo3(name = "clear")]
    fn py_clear(&self) -> PyResult<()> {
        Ok(self.clear()?)
    }

    fn __contains__(&self, name: &str) -> bool {
        self.exists(name)
    }

    fn __delitem__(&self, name: &str) -> PyResult<()> {
        Ok(self.delete(name)?)
    }

    fn __getitem__(&self, name: &str) -> PyResult<State> {
        Ok(self.load(name)?)
    }

    fn __iter__(&self) -> PyResult<StateIterator> {
        Ok(self.names()?)
    }

    fn __len__(&self) -> PyResult<usize> {
        Ok(self.count()?)
    }

    pub(crate) fn __repr__(&self) -> String {
        pyrepr!("StateRegistry" {
            state_dir = quote(self.state_dir().display()),
        })
    }
}
