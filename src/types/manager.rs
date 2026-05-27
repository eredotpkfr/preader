use std::{fs, path::PathBuf};

use anyhow::{Error, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    PReaderState,
    types::{
        config::{PReaderConfig, PReaderStateManagerConfig},
        state::PReaderStateData,
    },
};

pub(crate) const STATE_FILE_SUFFIX: &str = "state.json";
pub(crate) const TMP_STATE_FILE_SUFFIX: &str = "tmp";

#[derive(Clone, Serialize, Deserialize)]
pub struct PReaderStateManager {
    pub config: PReaderStateManagerConfig,
    pub last_saved_position: u64,
}

impl Default for PReaderStateManager {
    fn default() -> Self {
        Self {
            config: PReaderStateManagerConfig::default(),
            last_saved_position: 0,
        }
    }
}

impl From<PReaderConfig> for PReaderStateManager {
    fn from(config: PReaderConfig) -> Self {
        Self {
            config: config.into(),
            last_saved_position: 0,
        }
    }
}

impl PReaderStateManager {
    pub(crate) fn name(&self, file: &PathBuf) -> String {
        hex::encode(&Sha256::digest(file.as_os_str().as_encoded_bytes()))
    }

    pub(crate) fn path(&self, name: String) -> PathBuf {
        self.config.state_dir.join(name).with_added_extension(STATE_FILE_SUFFIX)
    }

    pub(crate) fn tmp(&self, name: String) -> PathBuf {
        self.config
            .state_dir
            .join(name)
            .with_added_extension(Utc::now().timestamp_nanos_opt().unwrap_or(0).to_string())
            .with_added_extension(STATE_FILE_SUFFIX)
            .with_added_extension(TMP_STATE_FILE_SUFFIX)
    }

    pub(crate) fn load(&self, name: &str) -> Result<PReaderState, Error> {
        let path = self.path(name.to_string());

        if !path.exists() {
            return Err(anyhow!("state not found: {name}"));
        }

        let content = fs::read_to_string(&path)?;
        let data: PReaderStateData = serde_json::from_str(&content)?;
        let state = PReaderState::from((data, self.clone()));

        if self.config.verify_state {
            state.verify()?;
        }

        Ok(state)
    }
}
