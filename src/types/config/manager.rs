use std::path::PathBuf;

use crate::types::config::reader::PReaderConfig;

#[derive(Clone)]
pub struct PReaderStateManagerConfig {
    pub state_dir: PathBuf,
}

impl From<PReaderConfig> for PReaderStateManagerConfig {
    fn from(config: PReaderConfig) -> Self {
        Self {
            state_dir: config.state_dir,
        }
    }
}
