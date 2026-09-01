use std::path::PathBuf;

use pyo3::prelude::*;
use serde::Deserialize;

use crate::utils::path::default_state_dir;

// std::io::BufReader default = 8 KiB; we use 64 KiB to amortize syscalls on
// large files where preader is typically used
pub const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024;
// Auto save state every 100 MiB of progress when `auto_save_state` is active
pub const DEFAULT_AUTO_SAVE_STATE_BYTES: u64 = 100 * 1024 * 1024;
// Verify saved state on load + explicit state resume by default
pub const DEFAULT_VERIFY_STATE: bool = true;

#[pyclass(module = "preader", from_py_object)]
#[derive(Clone, Deserialize)]
pub struct Config {
    #[pyo3(get)]
    pub buffer_capacity: usize,
    #[pyo3(get)]
    pub state_dir: PathBuf,
    #[pyo3(get)]
    pub auto_save_state: bool,
    #[pyo3(get)]
    pub auto_save_state_bytes: u64,
    #[pyo3(get)]
    pub auto_load_state: bool,
    #[pyo3(get)]
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

#[pymethods]
impl Config {
    #[new]
    #[pyo3(signature = (
        *,
        buffer_capacity = DEFAULT_BUFFER_CAPACITY,
        state_dir = default_state_dir(),
        auto_save_state = false,
        auto_save_state_bytes = DEFAULT_AUTO_SAVE_STATE_BYTES,
        auto_load_state = false,
        verify_state = DEFAULT_VERIFY_STATE,
    ))]
    pub fn new(
        buffer_capacity: usize,
        state_dir: PathBuf,
        auto_save_state: bool,
        auto_save_state_bytes: u64,
        auto_load_state: bool,
        verify_state: bool,
    ) -> Self {
        Self {
            buffer_capacity,
            state_dir,
            auto_save_state,
            auto_save_state_bytes,
            auto_load_state,
            verify_state,
        }
    }

    pub fn __repr__(&self) -> String {
        crate::macros::pyrepr!("Config" {
            state_dir = format!("'{}'", self.state_dir.display()),
            buffer_capacity = self.buffer_capacity,
            auto_save_state = self.auto_save_state,
            auto_save_state_bytes = self.auto_save_state_bytes,
            auto_load_state = self.auto_load_state,
            verify_state = self.verify_state,
        })
    }
}
