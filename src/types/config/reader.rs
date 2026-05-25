use std::path::PathBuf;

use pyo3::prelude::*;
use serde::Deserialize;

// std::io::BufReader default = 8 KiB; we use 64 KiB to amortize syscalls on
// large files where preader is typically used
const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024;
const DEFAULT_STATE_DIRECTORY: &str = "preader";

#[pyclass(from_py_object)]
#[derive(Clone, Deserialize)]
pub struct PReaderConfig {
    #[pyo3(get)]
    pub buffer_capacity: usize,
    #[pyo3(get)]
    pub state_dir: PathBuf,
    #[pyo3(get)]
    pub auto_save_state: bool,
    #[pyo3(get)]
    pub auto_load_state: bool,
}

impl Default for PReaderConfig {
    fn default() -> Self {
        Self {
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
            state_dir: dirs::cache_dir().unwrap_or_default().join(DEFAULT_STATE_DIRECTORY),
            auto_save_state: false,
            auto_load_state: false,
        }
    }
}

#[pymethods]
impl PReaderConfig {
    #[new]
    #[pyo3(signature = (
        buffer_capacity = DEFAULT_BUFFER_CAPACITY,
        state_dir = dirs::cache_dir().unwrap_or_default().join(DEFAULT_STATE_DIRECTORY),
        auto_save_state = false,
        auto_load_state = false,
    ))]
    pub fn new(
        buffer_capacity: usize,
        state_dir: PathBuf,
        auto_save_state: bool,
        auto_load_state: bool,
    ) -> Self {
        Self {
            buffer_capacity,
            state_dir,
            auto_save_state,
            auto_load_state,
        }
    }
}
