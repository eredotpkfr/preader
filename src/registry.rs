use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::to_string_pretty;
use time::UtcDateTime;

use crate::{
    Error, Mismatch, Result, State, StateIterator,
    constants::{CURRENT_DIR, STATE_FILE_EXTENSION, TMP_FILE_EXTENSION},
    macros::ensure,
    types::config::Config,
    utils::{file, hash, path},
};

#[cfg_attr(feature = "python", pyo3::pyclass(module = "preader"))]
#[derive(Clone, Debug)]
pub struct StateRegistry {
    pub(crate) state_dir: PathBuf,
    pub(crate) verify_state: bool,
    pub(crate) auto_load_state: bool,
}

impl Default for StateRegistry {
    fn default() -> Self {
        Self::from(&Config::default())
    }
}

impl From<&Config> for StateRegistry {
    fn from(config: &Config) -> Self {
        Self {
            state_dir: if config.state_dir.is_empty() {
                CURRENT_DIR.into()
            } else {
                config.state_dir.clone()
            },
            verify_state: config.verify_state,
            auto_load_state: config.auto_load_state,
        }
    }
}

impl StateRegistry {
    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    pub fn names(&self) -> Result<StateIterator> {
        StateIterator::new(self, None)
    }

    pub fn search(&self, pattern: &str) -> Result<StateIterator> {
        StateIterator::new(self, Some(pattern))
    }

    pub fn load(&self, name: &str) -> Result<State> {
        self.verified(self.read(name)?)
    }

    pub fn find(&self, name: &str) -> Option<State> {
        self.load(name).ok()
    }

    pub fn all(&self) -> Result<Vec<State>> {
        self.names()?.map(|name| self.load(&name?)).collect()
    }

    pub fn exists(&self, name: &str) -> bool {
        self.path(name).is_ok_and(|path| path.is_file())
    }

    pub fn count(&self) -> Result<usize> {
        self.names()?.map(|name| name.map(|_| 1)).sum()
    }

    pub fn autoname(&self, file: &Path) -> String {
        hash::sha256(file.as_os_str().as_encoded_bytes())
    }

    pub fn path(&self, name: &str) -> Result<PathBuf> {
        self.locate(name, &[STATE_FILE_EXTENSION])
    }

    pub fn resync(&self, name: &str, path: impl AsRef<Path>) -> Result<State> {
        let path = dunce::canonicalize(path)?;
        let resynced = self.read(name)?.resynced(&path, self.verify_state)?;

        self.commit(&resynced)?;

        Ok(resynced)
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        fs::remove_file(self.path(name)?).map_err(Error::not_found(name))
    }

    pub fn clear(&self) -> Result<()> {
        self.names()?.try_for_each(|name| self.delete(&name?))
    }

    #[cfg(feature = "testing")]
    pub fn tmp_path(&self, name: &str) -> Result<PathBuf> {
        self.tmp(name)
    }

    pub(crate) fn commit(&self, state: &State) -> Result<()> {
        let path = self.path(&state.name)?;
        let tmp = self.tmp(&state.name)?;

        Ok(file::write_atomic(&path, &tmp, to_string_pretty(state)?)?)
    }

    pub(crate) fn read(&self, name: &str) -> Result<State> {
        let path = self.path(name)?;

        ensure!(path.is_file(), Error::NotFound(name.to_owned()));

        let content = fs::read(&path).map_err(Error::not_found(name))?;
        let state: State = serde_json::from_slice(&content).map_err(Error::corrupt(name))?;

        ensure!(
            state.name == name,
            Error::mismatch(name, Mismatch::Name { saved: state.name })
        );

        Ok(state)
    }

    pub(crate) fn tracked(&self, state: State, file: &Path) -> Result<State> {
        self.verified(state.tracking(file, self.verify_state)?)
    }

    pub(crate) fn verified(&self, state: State) -> Result<State> {
        if self.verify_state {
            state.verify()?;
        }

        Ok(state)
    }

    fn tmp(&self, name: &str) -> Result<PathBuf> {
        let stamp = UtcDateTime::now().unix_timestamp_nanos().to_string();
        let extensions = &[&stamp, STATE_FILE_EXTENSION, TMP_FILE_EXTENSION];

        self.locate(name, extensions)
    }

    fn locate(&self, name: &str, extensions: &[&str]) -> Result<PathBuf> {
        let mut path = path::scoped_join(&self.state_dir, name)?;

        for extension in extensions {
            path.add_extension(extension);
        }

        let resolved = path::resolves_in_place(&self.state_dir, &path);

        ensure!(resolved, Error::alias(name));

        Ok(path)
    }
}
