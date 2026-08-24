use std::{fs, path::PathBuf};

use pyo3::{exceptions::PyKeyError, prelude::*};

use crate::{
    State, StateError, iterators::state::StateIterator, manager::StateManager,
    types::config::reader::Config,
};

#[pyclass]
pub struct StateRegistry {
    manager: StateManager,
}

impl From<&Config> for StateRegistry {
    fn from(config: &Config) -> Self {
        Self {
            manager: config.into(),
        }
    }
}

#[pymethods]
impl StateRegistry {
    fn __iter__(&self) -> PyResult<StateIterator> {
        self.names()
    }

    fn __getitem__(&self, name: &str) -> PyResult<State> {
        self.load(name)
    }

    fn __contains__(&self, name: &str) -> bool {
        self.exists(name)
    }

    fn __len__(&self) -> PyResult<usize> {
        self.names()?.try_fold(0_usize, |acc, name| name.map(|_| acc + 1))
    }

    fn __delitem__(&self, name: &str) -> PyResult<()> {
        self.delete(name)
    }

    fn names(&self) -> PyResult<StateIterator> {
        StateIterator::new(&self.manager.config.state_dir, None)
    }

    fn load(&self, name: &str) -> PyResult<State> {
        if !self.exists(name) {
            return Err(PyKeyError::new_err(name.to_string()));
        }

        self.manager.load(name).map_err(StateError::from_anyhow)
    }

    fn find(&self, name: &str) -> PyResult<Option<State>> {
        self.exists(name).then(|| self.load(name)).transpose()
    }

    fn search(&self, pattern: &str) -> PyResult<StateIterator> {
        StateIterator::new(&self.manager.config.state_dir, Some(pattern))
    }

    fn delete(&self, name: &str) -> PyResult<()> {
        if !self.exists(name) {
            return Err(PyKeyError::new_err(name.to_string()));
        }

        fs::remove_file(self.path(name)?).map_err(StateError::from_io)
    }

    fn exists(&self, name: &str) -> bool {
        self.manager.path(name).map(|path| path.exists()).unwrap_or(false)
    }

    fn all(&self) -> PyResult<Vec<State>> {
        self.names()?.map(|name| self.load(&name?)).collect()
    }

    fn clear(&self) -> PyResult<()> {
        self.names()?.try_for_each(|name| self.delete(&name?))
    }

    fn path(&self, name: &str) -> PyResult<PathBuf> {
        self.manager.path(name).map_err(StateError::from_anyhow)
    }
}
