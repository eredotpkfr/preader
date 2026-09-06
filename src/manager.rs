use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::{
    Error, PathError, Result, State,
    types::{
        config::{manager::StateManagerConfig, reader::Config},
        state::StateData,
    },
    utils::path::{has_no_symlinks, path_stem, scoped_join},
};

pub const STATE_FILE_SUFFIX: &str = "state.json";
pub const TMP_STATE_FILE_SUFFIX: &str = "tmp";

#[derive(Clone, Debug, Default)]
pub struct StateManager {
    pub config: StateManagerConfig,
    pub last_saved_position: u64,
}

impl From<&Config> for StateManager {
    fn from(config: &Config) -> Self {
        Self {
            config: config.into(),
            last_saved_position: 0,
        }
    }
}

impl StateManager {
    pub fn name(&self, file: &Path) -> String {
        hex::encode(Sha256::digest(file.as_os_str().as_encoded_bytes()))
    }

    pub fn path(&self, name: &str) -> Result<PathBuf> {
        let path = scoped_join(&self.config.state_dir, &path_stem(name, STATE_FILE_SUFFIX))?
            .with_added_extension(STATE_FILE_SUFFIX);

        has_no_symlinks(&self.config.state_dir, &path)
            .then_some(path)
            .ok_or_else(|| PathError::Symlink(name.to_owned()).into())
    }

    pub fn tmp(&self, name: &str) -> Result<PathBuf> {
        let stamp = Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let path = scoped_join(&self.config.state_dir, &path_stem(name, STATE_FILE_SUFFIX))?
            .with_added_extension(stamp.to_string())
            .with_added_extension(STATE_FILE_SUFFIX)
            .with_added_extension(TMP_STATE_FILE_SUFFIX);

        has_no_symlinks(&self.config.state_dir, &path)
            .then_some(path)
            .ok_or_else(|| PathError::Symlink(name.to_owned()).into())
    }

    pub fn load(&self, name: &str) -> Result<State> {
        let path = self.path(name)?;

        if !path.is_file() {
            return Err(Error::Missing(name.to_owned()));
        }

        let content = fs::read_to_string(&path)?;
        let data: StateData = serde_json::from_str(&content)?;
        let state = State::from((data, self.clone()));

        if self.config.verify_state {
            state.verify()?;
        }

        Ok(state)
    }
}
