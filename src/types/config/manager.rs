use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::types::config::reader::{DEFAULT_STATE_DIRECTORY, PReaderConfig};

#[derive(Clone, Serialize, Deserialize)]
pub struct PReaderStateManagerConfig {
    pub state_dir: PathBuf,
}

impl Default for PReaderStateManagerConfig {
    fn default() -> Self {
        Self {
            state_dir: dirs::cache_dir().unwrap_or_default().join(DEFAULT_STATE_DIRECTORY),
        }
    }
}

impl From<PReaderConfig> for PReaderStateManagerConfig {
    fn from(config: PReaderConfig) -> Self {
        Self {
            state_dir: config.state_dir,
        }
    }
}
