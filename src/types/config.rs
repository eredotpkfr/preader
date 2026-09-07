use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    constants::{DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, DEFAULT_VERIFY_STATE},
    utils::path::default_state_dir,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Config {
    pub buffer_capacity: usize,
    pub state_dir: PathBuf,
    pub auto_save_state: bool,
    pub auto_save_state_bytes: u64,
    pub auto_load_state: bool,
    pub verify_state: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
            state_dir: default_state_dir(),
            auto_save_state: false,
            auto_save_state_bytes: DEFAULT_AUTO_SAVE_STATE_BYTES,
            auto_load_state: false,
            verify_state: DEFAULT_VERIFY_STATE,
        }
    }
}
