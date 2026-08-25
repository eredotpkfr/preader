use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Error, anyhow};
use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::{
    State,
    types::{
        config::{manager::StateManagerConfig, reader::Config},
        state::StateData,
    },
    utils::path::{scoped_join, strip_extension},
};

pub const STATE_FILE_SUFFIX: &str = "state.json";
pub const TMP_STATE_FILE_SUFFIX: &str = "tmp";

#[derive(Clone, Default)]
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

    pub fn path(&self, name: &str) -> Result<PathBuf, Error> {
        let path = scoped_join(
            &self.config.state_dir,
            strip_extension(name, STATE_FILE_SUFFIX),
        )?;

        Ok(path.with_added_extension(STATE_FILE_SUFFIX))
    }

    pub fn tmp(&self, name: &str) -> Result<PathBuf, Error> {
        let path = scoped_join(
            &self.config.state_dir,
            strip_extension(name, STATE_FILE_SUFFIX),
        )?;
        let timestamp_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);

        Ok(path
            .with_added_extension(timestamp_nanos.to_string())
            .with_added_extension(STATE_FILE_SUFFIX)
            .with_added_extension(TMP_STATE_FILE_SUFFIX))
    }

    pub fn load(&self, name: &str) -> Result<State, Error> {
        let path = self.path(name)?;

        if !path.is_file() {
            return Err(anyhow!("state not found: {name}"));
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
