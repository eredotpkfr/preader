use std::path::{Path, PathBuf};

use pyo3::prelude::*;

use crate::{Result, State, StateIterator, StateRegistry, python::macros::repr::pyrepr};

#[pymethods]
impl StateRegistry {
    #[getter]
    #[pyo3(name = "state_dir")]
    fn py_state_dir(&self) -> &Path {
        self.state_dir()
    }

    #[pyo3(name = "names")]
    fn py_names(&self) -> Result<StateIterator> {
        self.names()
    }

    #[pyo3(name = "search")]
    fn py_search(&self, pattern: &str) -> Result<StateIterator> {
        self.search(pattern)
    }

    #[pyo3(name = "load")]
    fn py_load(&self, name: &str) -> Result<State> {
        self.load(name)
    }

    #[pyo3(name = "find")]
    fn py_find(&self, name: &str) -> Option<State> {
        self.find(name)
    }

    #[pyo3(name = "all")]
    fn py_all(&self) -> Result<Vec<State>> {
        self.all()
    }

    #[pyo3(name = "exists")]
    fn py_exists(&self, name: &str) -> bool {
        self.exists(name)
    }

    #[pyo3(name = "path")]
    fn py_path(&self, name: &str) -> Result<PathBuf> {
        self.path(name)
    }

    #[pyo3(name = "resync")]
    fn py_resync(&self, name: &str, path: PathBuf) -> Result<State> {
        self.resync(name, path)
    }

    #[pyo3(name = "delete")]
    fn py_delete(&self, name: &str) -> Result<()> {
        self.delete(name)
    }

    #[pyo3(name = "clear")]
    fn py_clear(&self) -> Result<()> {
        self.clear()
    }

    fn __contains__(&self, name: &str) -> bool {
        self.exists(name)
    }

    fn __delitem__(&self, name: &str) -> Result<()> {
        self.delete(name)
    }

    fn __getitem__(&self, name: &str) -> Result<State> {
        self.load(name)
    }

    fn __iter__(&self) -> Result<StateIterator> {
        self.names()
    }

    fn __len__(&self) -> Result<usize> {
        self.count()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, state_dir)
    }
}
