use std::{fs, path::PathBuf};

use pyo3::{exceptions::PyValueError, prelude::*};
use sha2::{Digest, Sha256};

use crate::{
    PReaderState,
    types::config::{PReaderConfig, PReaderStateManagerConfig},
};

#[pyclass]
pub struct PReaderStateManager {
    pub config: PReaderStateManagerConfig,
}

impl From<PReaderConfig> for PReaderStateManager {
    fn from(config: PReaderConfig) -> Self {
        Self {
            config: config.into(),
        }
    }
}

impl PReaderStateManager {
    pub(crate) fn name(&self, path: &PathBuf) -> String {
        hex::encode(&Sha256::digest(path.as_os_str().as_encoded_bytes()))
    }

    pub(crate) fn path(&self, name: String) -> PathBuf {
        self.config.state_dir.join(name)
    }

    pub(crate) fn load(&self, name: String) -> PyResult<PReaderState> {
        let path = self.path(name);

        if !path.exists() {
            return Err(PyValueError::new_err("state not found"));
        }

        let content =
            fs::read_to_string(&path).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let state: PReaderState =
            serde_json::from_str(&content).map_err(|_| PyValueError::new_err("state not found"))?;

        // if verify {
        //     let computed = state.compute_checksum();
        //     if computed != state.checksum {
        //         return Err(PyValueError::new_err("state not found"));
        //     }
        // }
        Ok(state)
    }
}
