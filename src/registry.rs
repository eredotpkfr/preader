use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{Error, Result, State, StateIterator, manager::StateManager, types::config::Config};

#[derive(Debug)]
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

impl StateRegistry {
    pub fn state_dir(&self) -> &Path {
        &self.manager.state_dir
    }

    pub fn names(&self) -> Result<StateIterator> {
        StateIterator::new(self.state_dir(), None)
    }

    pub fn search(&self, pattern: &str) -> Result<StateIterator> {
        StateIterator::new(self.state_dir(), Some(pattern))
    }

    pub fn load(&self, name: &str) -> Result<State> {
        self.manager.load(name)
    }

    pub fn find(&self, name: &str) -> Option<State> {
        self.load(name).ok()
    }

    pub fn all(&self) -> Result<Vec<State>> {
        self.names()?.map(|name| self.load(&name?)).collect()
    }

    pub fn exists(&self, name: &str) -> bool {
        self.manager.path(name).is_ok_and(|path| path.exists())
    }

    pub fn count(&self) -> Result<usize> {
        self.names()?.try_fold(0_usize, |total, name| name.map(|_| total + 1))
    }

    pub fn path(&self, name: &str) -> Result<PathBuf> {
        self.manager.path(name)
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        let path = self.path(name)?;

        if !path.exists() {
            return Err(Error::NotFound(name.to_owned()));
        }

        Ok(fs::remove_file(path)?)
    }

    pub fn clear(&self) -> Result<()> {
        self.names()?.try_for_each(|name| self.delete(&name?))
    }

    pub(crate) fn manager(&self) -> &StateManager {
        &self.manager
    }
}
