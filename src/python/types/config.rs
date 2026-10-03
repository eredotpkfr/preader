use std::path::PathBuf;

use pyo3::prelude::*;

use crate::{
    Config, DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, DEFAULT_VERIFY_STATE,
    python::{macros::repr::pyrepr, utils::text::quote},
    utils::path::default_state_dir,
};

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
    fn py_new(
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

    pub(crate) fn __repr__(&self) -> String {
        pyrepr!("Config" {
            state_dir = quote(self.state_dir.display()),
            buffer_capacity = self.buffer_capacity,
            auto_save_state = self.auto_save_state,
            auto_save_state_bytes = self.auto_save_state_bytes,
            auto_load_state = self.auto_load_state,
            verify_state = self.verify_state,
        })
    }
}
