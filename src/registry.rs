use std::{
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use walkdir::{IntoIter, WalkDir};

use crate::{
    Error, Result, State, manager::StateManager, types::config::reader::Config,
    utils::path::has_no_symlinks,
};

pub(crate) const STATE_FILE_EXT: &str = ".state.json";

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
        &self.manager.config.state_dir
    }

    pub fn names(&self) -> Result<StateIterator> {
        StateIterator::new(self.state_dir(), None)
    }

    pub fn search(&self, pattern: &str) -> Result<StateIterator> {
        StateIterator::new(self.state_dir(), Some(pattern))
    }

    pub fn load(&self, name: &str) -> Result<State> {
        if !self.exists(name) {
            return Err(Error::Missing(name.to_owned()));
        }

        self.manager.load(name)
    }

    pub fn find(&self, name: &str) -> Result<Option<State>> {
        self.exists(name).then(|| self.load(name)).transpose()
    }

    pub fn all(&self) -> Result<Vec<State>> {
        self.names()?.map(|name| self.load(&name?)).collect()
    }

    pub fn exists(&self, name: &str) -> bool {
        self.manager.path(name).map(|path| path.exists()).unwrap_or(false)
    }

    pub fn count(&self) -> Result<usize> {
        self.names()?.try_fold(0_usize, |total, name| name.map(|_| total + 1))
    }

    pub fn path(&self, name: &str) -> Result<PathBuf> {
        self.manager.path(name)
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        if !self.exists(name) {
            return Err(Error::Missing(name.to_owned()));
        }

        Ok(fs::remove_file(self.path(name)?)?)
    }

    pub fn clear(&self) -> Result<()> {
        self.names()?.try_for_each(|name| self.delete(&name?))
    }
}

#[derive(Debug)]
pub struct StateIterator {
    state_dir: PathBuf,
    entries: IntoIter,
    pattern: Option<Regex>,
}

impl StateIterator {
    pub(crate) fn new(dir: &Path, pattern: Option<&str>) -> Result<Self> {
        fs::create_dir_all(dir)?;

        Ok(Self {
            state_dir: dir.to_path_buf(),
            entries: WalkDir::new(dir).min_depth(1).into_iter(),
            pattern: pattern.map(Regex::new).transpose()?,
        })
    }
}

impl Iterator for StateIterator {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let (root, pattern) = (&self.state_dir, self.pattern.as_ref());

        self.entries.find_map(|entry| {
            let entry = match entry {
                Err(error) => return Some(Err(Error::Io(error.into()))),
                Ok(entry) => entry,
            };
            let path = entry.path();
            let name =
                path.strip_prefix(root).ok()?.to_str()?.strip_suffix(STATE_FILE_EXT)?.to_owned();

            (path.is_file()
                && has_no_symlinks(root, path)
                && pattern.is_none_or(|regex| regex.is_match(&name)))
            .then_some(Ok(name))
        })
    }
}
