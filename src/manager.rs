use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::{
    Error, PathError, Result, State,
    constants::{STATE_FILE_EXTENSION, TMP_FILE_EXTENSION},
    types::config::Config,
    utils::path::{has_no_symlinks, path_stem, scoped_join},
};

#[derive(Clone, Debug)]
pub struct StateManager {
    pub(crate) state_dir: PathBuf,
    pub(crate) verify_state: bool,
    pub(crate) auto_load_state: bool,
}

impl Default for StateManager {
    fn default() -> Self {
        Self::from(&Config::default())
    }
}

impl From<&Config> for StateManager {
    fn from(config: &Config) -> Self {
        Self {
            state_dir: config.state_dir.clone(),
            verify_state: config.verify_state,
            auto_load_state: config.auto_load_state,
        }
    }
}

impl StateManager {
    pub fn autoname(&self, file: &Path) -> String {
        hex::encode(Sha256::digest(file.as_os_str().as_encoded_bytes()))
    }

    pub fn path(&self, name: &str) -> Result<PathBuf> {
        self.locate(name, &[STATE_FILE_EXTENSION])
    }

    pub fn tmp(&self, name: &str) -> Result<PathBuf> {
        let stamp = Utc::now().timestamp_nanos_opt().unwrap_or(0).to_string();

        self.locate(name, &[&stamp, STATE_FILE_EXTENSION, TMP_FILE_EXTENSION])
    }

    pub fn load(&self, name: &str) -> Result<State> {
        let path = self.path(name)?;

        if !path.is_file() {
            return Err(Error::NotFound(name.to_owned()));
        }

        let content = fs::read_to_string(&path)?;
        let state = State::from((serde_json::from_str(&content)?, self.clone()));

        if self.verify_state {
            state.verify()?;
        }

        Ok(state)
    }

    fn locate(&self, name: &str, extensions: &[&str]) -> Result<PathBuf> {
        let mut path = scoped_join(&self.state_dir, &path_stem(name, STATE_FILE_EXTENSION))?;

        for extension in extensions {
            path = path.with_added_extension(extension);
        }

        has_no_symlinks(&self.state_dir, &path)
            .then_some(path)
            .ok_or_else(|| PathError::Symlink(name.to_owned()).into())
    }
}
