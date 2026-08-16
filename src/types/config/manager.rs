use std::path::PathBuf;

use crate::{
    types::config::reader::{Config, DEFAULT_VERIFY_STATE},
    utils::path::default_state_dir,
};

#[derive(Clone)]
pub struct StateManagerConfig {
    pub state_dir: PathBuf,
    pub verify_state: bool,
}

impl Default for StateManagerConfig {
    fn default() -> Self {
        Self {
            state_dir: default_state_dir(),
            verify_state: DEFAULT_VERIFY_STATE,
        }
    }
}

impl From<&Config> for StateManagerConfig {
    fn from(config: &Config) -> Self {
        Self {
            state_dir: config.state_dir.clone(),
            verify_state: config.verify_state,
        }
    }
}
