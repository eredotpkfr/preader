use std::{fs, path::PathBuf};

use anyhow::{Error, anyhow};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    PReaderState,
    types::config::{PReaderConfig, PReaderStateManagerConfig},
};

#[pyclass(from_py_object)]
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
        self.config.state_dir.join(name)
    }

    pub(crate) fn load(&self, file: &PathBuf) -> Result<PReaderState, Error> {
        let path = self.path(self.name(file));

        if !path.exists() {
            return Err(anyhow!("state not found"));
        }

        let content = fs::read_to_string(&path)?;
        let mut state: PReaderState = serde_json::from_str(&content)?;

        state.manager = self.clone();

        if self.config.verify_state {
            state.verify()?;
        }

        Ok(state)
    }
}
