use std::path::PathBuf;

use serde::Deserialize;

use crate::utils::path::default_state_dir;

// std::io::BufReader default = 8 KiB; we use 64 KiB to amortize syscalls on
// large files where preader is typically used
pub const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024;
// Auto save state every 100 MiB of progress when `auto_save_state` is active
pub const DEFAULT_AUTO_SAVE_STATE_BYTES: u64 = 100 * 1024 * 1024;
// Verify saved state on load + explicit state resume by default
pub const DEFAULT_VERIFY_STATE: bool = true;

#[derive(Clone, Debug, Deserialize, PartialEq)]
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
