use std::{fs, path::PathBuf};

use pyo3::prelude::*;

use crate::{
    Error, State, iterators::state::StateIterator, manager::StateManager,
    types::config::reader::Config,
};

#[pyclass(module = "preader")]
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
    fn __iter__(&self) -> Result<StateIterator, Error> {
        self.names()
    }

    fn __getitem__(&self, name: &str) -> Result<State, Error> {
        self.load(name)
    }

    fn __contains__(&self, name: &str) -> bool {
        self.exists(name)
    }

    fn __len__(&self) -> Result<usize, Error> {
        self.names()?.try_fold(0_usize, |acc, name| name.map(|_| acc + 1))
    }

    fn __delitem__(&self, name: &str) -> Result<(), Error> {
        self.delete(name)
    }

    fn names(&self) -> Result<StateIterator, Error> {
        StateIterator::new(&self.manager.config.state_dir, None)
    }

    fn load(&self, name: &str) -> Result<State, Error> {
        if !self.exists(name) {
            return Err(Error::Missing(name.to_string()));
        }

        self.manager.load(name)
    }

    fn find(&self, name: &str) -> Result<Option<State>, Error> {
        self.exists(name).then(|| self.load(name)).transpose()
    }

    fn search(&self, pattern: &str) -> Result<StateIterator, Error> {
        StateIterator::new(&self.manager.config.state_dir, Some(pattern))
    }

    fn delete(&self, name: &str) -> Result<(), Error> {
        if !self.exists(name) {
            return Err(Error::Missing(name.to_string()));
        }

        Ok(fs::remove_file(self.path(name)?)?)
    }

    fn exists(&self, name: &str) -> bool {
        self.manager.path(name).map(|path| path.exists()).unwrap_or(false)
    }

    fn all(&self) -> Result<Vec<State>, Error> {
        self.names()?.map(|name| self.load(&name?)).collect()
    }

    fn clear(&self) -> Result<(), Error> {
        self.names()?.try_for_each(|name| self.delete(&name?))
    }

    fn path(&self, name: &str) -> Result<PathBuf, Error> {
        self.manager.path(name)
    }
}
