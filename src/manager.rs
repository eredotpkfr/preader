use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::{
    Error, Mismatch, PathError, Result, State, StateData,
    constants::{STATE_FILE_EXTENSION, TMP_FILE_EXTENSION},
    macros::ensure,
    types::config::Config,
    utils::path::{resolves_in_place, scoped_join},
};

// STATE_FILE_EXTENSION past its leading dot, since add_extension inserts the dot itself
const STATE_FILE_EXTENSION_WITHOUT_DOT: &str = STATE_FILE_EXTENSION.split_at(1).1;

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
        self.locate(name, &[STATE_FILE_EXTENSION_WITHOUT_DOT])
    }

    pub fn tmp(&self, name: &str) -> Result<PathBuf> {
        let stamp = Utc::now().timestamp_nanos_opt().unwrap_or(0).to_string();
        let extensions = &[&stamp, STATE_FILE_EXTENSION_WITHOUT_DOT, TMP_FILE_EXTENSION];

        self.locate(name, extensions)
    }

    pub fn state(&self, data: StateData) -> State {
        State {
            data,
            manager: self.clone(),
        }
    }

    pub fn load(&self, name: &str) -> Result<State> {
        let path = self.path(name)?;

        ensure!(path.is_file(), Error::NotFound(name.to_owned()));

        let data: StateData = serde_json::from_str(&fs::read_to_string(&path)?)?;

        ensure!(
            data.name == name,
            Mismatch::Name {
                saved: data.name,
                current: name.to_owned()
            }
        );

        let state = self.state(data);

        if self.verify_state {
            state.verify()?;
        }

        Ok(state)
    }

    fn locate(&self, name: &str, extensions: &[&str]) -> Result<PathBuf> {
        let mut path = scoped_join(&self.state_dir, name)?;

        for extension in extensions {
            path.add_extension(extension);
        }

        ensure!(
            resolves_in_place(&self.state_dir, &path),
            PathError::Alias(name.to_owned())
        );

        Ok(path)
    }
}
