use std::{num::NonZeroU64, path::PathBuf};

use pyo3::prelude::*;

use crate::{
    Config, DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, DEFAULT_VERIFY_STATE,
    python::macros::repr::pyrepr, utils::path,
};

#[pymethods]
impl Config {
    /// Creates a configuration from keyword arguments; omitted ones keep their defaults
    ///
    /// By default states live in a `preader` directory under the user's cache directory, the read
    /// buffer holds 64 KiB and `verify_state` is on. Autosave runs every 100 MiB once
    /// `auto_save_state` is on. Raises `ValueError` when `auto_save_state_bytes` is 0
    #[new]
    #[pyo3(signature = (
        *,
        buffer_capacity = DEFAULT_BUFFER_CAPACITY,
        state_dir = path::default_state_dir(),
        auto_save_state = false,
        auto_save_state_bytes = DEFAULT_AUTO_SAVE_STATE_BYTES,
        auto_load_state = false,
        verify_state = DEFAULT_VERIFY_STATE,
    ))]
    fn py_new(
        buffer_capacity: usize,
        state_dir: PathBuf,
        auto_save_state: bool,
        auto_save_state_bytes: NonZeroU64,
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

    /// Returns the class name with every setting
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(
            slf,
            state_dir,
            buffer_capacity,
            auto_save_state,
            auto_save_state_bytes,
            auto_load_state,
            verify_state
        )
    }
}
