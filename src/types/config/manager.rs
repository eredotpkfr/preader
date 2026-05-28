use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    types::config::reader::{DEFAULT_VERIFY_STATE, PReaderConfig},
    utils::default_state_dir,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct PReaderStateManagerConfig {
    pub state_dir: PathBuf,
    pub verify_state: bool,
}

impl Default for PReaderStateManagerConfig {
    fn default() -> Self {
        Self {
            state_dir: default_state_dir(),
            verify_state: DEFAULT_VERIFY_STATE,
        }
    }
}

impl From<&PReaderConfig> for PReaderStateManagerConfig {
    fn from(config: &PReaderConfig) -> Self {
        Self {
            state_dir: config.state_dir.clone(),
            verify_state: config.verify_state,
        }
    }
}
